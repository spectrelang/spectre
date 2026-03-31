use crate::module::ResolvedModule;
use crate::parser::{Expr, Field, FnDef, Item, Stmt, TypeExpr};
use std::collections::HashMap;

fn qbe_type(ty: &TypeExpr) -> &'static str {
    match ty {
        TypeExpr::Named(n) => match n.as_str() {
            "i32" | "u32" | "bool" => "w",
            "i64" | "u64" | "usize" => "l",
            "f32" => "s",
            "f64" => "d",
            _ => "l",
        },
        TypeExpr::Slice(_) => "l",
        TypeExpr::Ref(_) => "l",
        TypeExpr::Option(_) => "l",
        TypeExpr::Void => "w",
        TypeExpr::Untyped => "l",
    }
}

pub struct Codegen {
    out: String,
    data: Vec<(String, String)>,
    str_counter: usize,
    tmp_counter: usize,
    locals: HashMap<String, String>,
    local_mutability: HashMap<String, bool>,
    local_type_annotations: HashMap<String, String>,
    type_defs: HashMap<String, Vec<Field>>,
    trusted_fns: std::collections::HashSet<String>,
    current_fn: String,
}

impl Codegen {
    pub fn new() -> Self {
        Self {
            out: String::new(),
            data: Vec::new(),
            str_counter: 0,
            tmp_counter: 0,
            locals: HashMap::new(),
            local_mutability: HashMap::new(),
            local_type_annotations: HashMap::new(),
            type_defs: HashMap::new(),
            trusted_fns: std::collections::HashSet::new(),
            current_fn: String::new(),
        }
    }

    pub fn finish(mut self) -> String {
        let mut data_section = String::new();
        for (label, value) in &self.data {
            data_section.push_str(&format!("data ${label} = {{ b \"{value}\", b 0 }}\n"));
        }
        if !data_section.is_empty() {
            self.out.push('\n');
            self.out.push_str(&data_section);
        }
        self.out
    }

    fn fresh_tmp(&mut self) -> String {
        let n = self.tmp_counter;
        self.tmp_counter += 1;
        format!("%t{n}")
    }

    fn intern_string(&mut self, s: &str) -> String {
        let label = format!("str{}", self.str_counter);
        self.str_counter += 1;
        let escaped = s
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n");
        self.data.push((label.clone(), escaped));
        label
    }

    fn emit(&mut self, s: &str) {
        self.out.push_str(s);
        self.out.push('\n');
    }

    pub fn emit_module(&mut self, resolved: &ResolvedModule) -> Result<(), String> {
        let ns = build_namespace(resolved);
        let trusted = build_trusted_set(resolved);
        self.trusted_fns = trusted;
        self.emit_module_recursive(resolved, &ns)
    }

    fn emit_module_recursive(
        &mut self,
        resolved: &ResolvedModule,
        ns: &Namespace,
    ) -> Result<(), String> {
        for child in resolved.imports.values() {
            self.emit_module_recursive(child, ns)?;
        }

        for item in &resolved.ast.items {
            if let Item::TypeDef { name, fields } = item {
                self.type_defs.insert(name.clone(), fields.clone());
            }
        }

        for item in &resolved.ast.items {
            match item {
                Item::Fn(f) => self.emit_fn(f, ns)?,
                Item::Use { .. } | Item::Const { .. } | Item::TypeDef { .. } => {}
            }
        }
        Ok(())
    }

    fn emit_fn(&mut self, f: &FnDef, ns: &Namespace) -> Result<(), String> {
        self.locals.clear();
        self.local_mutability.clear();
        self.local_type_annotations.clear();
        self.tmp_counter = 0;
        self.current_fn = f.name.clone();

        if !f.trusted {
            let has_pre = f.body.iter().any(|s| matches!(s, Stmt::Pre(_)));
            let has_post = f.body.iter().any(|s| matches!(s, Stmt::Post(_)));
            let all_trusted = f.body.iter().all(|s| match s {
                Stmt::Assign { .. } => false,
                Stmt::Expr(Expr::Trust(_)) => true,
                Stmt::Expr(Expr::Call { callee, .. }) => {
                    let path = expr_to_path(callee);
                    !path.is_empty()
                }
                Stmt::Pre(_) | Stmt::Post(_) => true,
                _ => false,
            });
            if !all_trusted && (!has_pre || !has_post) {
                return Err(format!(
                    "pure function '{}' must have both 'pre' and 'post' contract blocks, or consist entirely of 'trust' statements",
                    f.name
                ));
            }
        }

        let export = if f.public { "export " } else { "" };
        let ret_ty = match &f.ret {
            TypeExpr::Void => String::new(),
            ty => format!("{} ", qbe_type(ty)),
        };

        let params: Vec<String> = f
            .params
            .iter()
            .map(|(name, ty)| {
                let tmp = format!("%{name}");
                self.locals.insert(name.clone(), tmp.clone());
                format!("{} {tmp}", qbe_type(ty))
            })
            .collect();

        self.emit(&format!(
            "{export}function {ret_ty}${name}({params}) {{",
            name = f.name,
            params = params.join(", ")
        ));
        self.emit("@start");

        for stmt in &f.body {
            self.emit_stmt(stmt, ns, &f.ret)?;
        }

        if matches!(f.ret, TypeExpr::Void) {
            self.emit("    ret");
        }

        self.emit("}");
        self.emit("");
        Ok(())
    }

    fn emit_stmt(&mut self, stmt: &Stmt, ns: &Namespace, ret_ty: &TypeExpr) -> Result<(), String> {
        match stmt {
            Stmt::Val {
                name,
                mutable,
                expr,
                ty,
            } => {
                let (tmp, _) = self.emit_expr(expr, ns)?;
                self.locals.insert(name.clone(), tmp);
                self.local_mutability.insert(name.clone(), *mutable);
                if let Some(TypeExpr::Named(type_name)) = ty {
                    self.local_type_annotations
                        .insert(name.clone(), type_name.clone());
                }
            }

            Stmt::Assign { target, value } => {
                if let Some(root) = expr_root_name(target) {
                    let is_mut = self.local_mutability.get(&root).copied().unwrap_or(false);
                    if !is_mut {
                        return Err(format!(
                            "cannot assign to field of immutable binding '{root}'"
                        ));
                    }
                }
                if let Expr::Field(base, field_name) = target {
                    if let Ok(type_name) = self.infer_struct_type_name(base) {
                        if let Some(fields) = self.type_defs.get(&type_name) {
                            if let Some(field) = fields.iter().find(|f| f.name == *field_name) {
                                if !field.mutable {
                                    return Err(format!(
                                        "cannot assign to immutable field '{field_name}' of type '{type_name}'"
                                    ));
                                }
                            }
                        }
                    }
                }
                let (val_tmp, val_ty) = self.emit_expr(value, ns)?;
                let store_val = if val_ty == "l" {
                    let w = self.fresh_tmp();
                    self.emit(&format!("    {w} =w copy {val_tmp}"));
                    w
                } else {
                    val_tmp
                };
                let ptr = self.emit_field_ptr(target, ns)?;
                self.emit(&format!("    storew {store_val}, {ptr}"));
            }

            Stmt::Return(None) => {
                self.emit("    ret");
            }
            Stmt::Return(Some(expr)) => {
                let (tmp, _) = self.emit_expr(expr, ns)?;
                self.emit(&format!("    ret {tmp}"));
            }
            Stmt::Expr(expr) => {
                self.emit_expr(expr, ns)?;
            }
            Stmt::Pre(contracts) => {
                for c in contracts {
                    let (cond, _) = self.emit_expr(&c.expr, ns)?;
                    let ok_lbl = format!("@pre_ok_{}", self.tmp_counter);
                    let fail_lbl = format!("@pre_fail_{}", self.tmp_counter);
                    self.tmp_counter += 1;
                    self.emit(&format!("    jnz {cond}, {ok_lbl}, {fail_lbl}"));
                    // failure block: print message then abort
                    self.emit(&format!("{fail_lbl}"));
                    let msg = match &c.label {
                        Some(lbl) => format!(
                            "spectre: precondition '{}' violated in function '{}'\n",
                            lbl, self.current_fn
                        ),
                        None => format!(
                            "spectre: precondition violated in function '{}'\n",
                            self.current_fn
                        ),
                    };
                    let msg_label = self.intern_string(&msg);
                    let msg_tmp = self.fresh_tmp();
                    self.emit(&format!("    {msg_tmp} =l copy ${msg_label}"));
                    self.emit(&format!("    call $dprintf(w 2, l {msg_tmp})"));
                    self.emit("    call $abort()");
                    self.emit("    hlt");
                    self.emit(&format!("{ok_lbl}"));
                }
            }
            Stmt::Post(contracts) => {
                for c in contracts {
                    let (cond, _) = self.emit_expr(&c.expr, ns)?;
                    let ok_lbl = format!("@post_ok_{}", self.tmp_counter);
                    let fail_lbl = format!("@post_fail_{}", self.tmp_counter);
                    self.tmp_counter += 1;
                    self.emit(&format!("    jnz {cond}, {ok_lbl}, {fail_lbl}"));
                    self.emit(&format!("{fail_lbl}"));
                    let msg = match &c.label {
                        Some(lbl) => format!(
                            "spectre: postcondition '{}' violated in function '{}'\n",
                            lbl, self.current_fn
                        ),
                        None => format!(
                            "spectre: postcondition violated in function '{}'\n",
                            self.current_fn
                        ),
                    };
                    let msg_label = self.intern_string(&msg);
                    let msg_tmp = self.fresh_tmp();
                    self.emit(&format!("    {msg_tmp} =l copy ${msg_label}"));
                    self.emit(&format!("    call $dprintf(w 2, l {msg_tmp})"));
                    self.emit("    call $abort()");
                    self.emit("    hlt");
                    self.emit(&format!("{ok_lbl}"));
                }
            }
            Stmt::If { cond, then, else_ } => {
                let (cond_tmp, _) = self.emit_expr(cond, ns)?;
                let then_lbl = format!("@if_then_{}", self.tmp_counter);
                let else_lbl = format!("@if_else_{}", self.tmp_counter);
                let end_lbl = format!("@if_end_{}", self.tmp_counter);
                self.tmp_counter += 1;

                if else_.is_some() {
                    self.emit(&format!("    jnz {cond_tmp}, {then_lbl}, {else_lbl}"));
                } else {
                    self.emit(&format!("    jnz {cond_tmp}, {then_lbl}, {end_lbl}"));
                }

                self.emit(&format!("{then_lbl}"));
                for s in then {
                    self.emit_stmt(s, ns, ret_ty)?;
                }
                self.emit(&format!("    jmp {end_lbl}"));

                if let Some(else_stmts) = else_ {
                    self.emit(&format!("{else_lbl}"));
                    for s in else_stmts {
                        self.emit_stmt(s, ns, ret_ty)?;
                    }
                    self.emit(&format!("    jmp {end_lbl}"));
                }

                self.emit(&format!("{end_lbl}"));
            }
        }
        Ok(())
    }

    /// Compute a pointer to a field within a struct.
    /// `expr` must be of the form `base.field` or `base.field.field...`
    fn emit_field_ptr(&mut self, expr: &Expr, ns: &Namespace) -> Result<String, String> {
        match expr {
            Expr::Field(base, field_name) => {
                let base_ptr = match base.as_ref() {
                    Expr::Ident(name) => self
                        .locals
                        .get(name)
                        .cloned()
                        .ok_or_else(|| format!("undefined variable: {name}"))?,
                    other => self.emit_field_ptr(other, ns)?,
                };

                let offset = self.field_offset_for(base, field_name)?;
                let ptr = self.fresh_tmp();
                self.emit(&format!("    {ptr} =l add {base_ptr}, {offset}"));
                Ok(ptr)
            }
            _ => Err("expected field access expression for assignment target".into()),
        }
    }

    /// Return the byte offset of `field_name` within the struct that `base` refers to.
    /// We look up the binding's declared type annotation to find the type definition.
    fn field_offset_for(&self, base: &Expr, field_name: &str) -> Result<usize, String> {
        let type_name = self.infer_struct_type_name(base)?;
        let fields = self
            .type_defs
            .get(&type_name)
            .ok_or_else(|| format!("unknown type '{type_name}'"))?;
        fields
            .iter()
            .position(|f| f.name == field_name)
            .map(|i| i * 8)
            .ok_or_else(|| format!("type '{type_name}' has no field '{field_name}'"))
    }

    /// Try to infer the struct type name of an expression (best-effort, ident only).
    fn infer_struct_type_name(&self, expr: &Expr) -> Result<String, String> {
        match expr {
            Expr::Ident(name) => self
                .local_type_annotations
                .get(name)
                .cloned()
                .ok_or_else(|| format!("cannot determine type of '{name}'")),
            _ => Err("cannot determine struct type for complex expression".into()),
        }
    }

    /// Promote a w value to l via sign-extension (needed for variadic call args).
    fn promote_to_l(&mut self, tmp: String, ty: &'static str) -> (String, &'static str) {
        if ty == "w" {
            let ext = self.fresh_tmp();
            self.emit(&format!("    {ext} =l extsw {tmp}"));
            (ext, "l")
        } else {
            (tmp, ty)
        }
    }

    fn emit_expr(&mut self, expr: &Expr, ns: &Namespace) -> Result<(String, &'static str), String> {
        match expr {
            Expr::IntLit(n) => Ok((n.to_string(), "w")),

            Expr::StrLit(s) => {
                let label = self.intern_string(s);
                let tmp = self.fresh_tmp();
                self.emit(&format!("    {tmp} =l copy ${label}"));
                Ok((tmp, "l"))
            }

            Expr::Ident(name) => self
                .locals
                .get(name)
                .cloned()
                .map(|t| (t, "l"))
                .ok_or_else(|| format!("undefined variable: {name}")),

            Expr::Bool(b) => Ok((if *b { "1".into() } else { "0".into() }, "w")),
            Expr::None => Ok(("0".into(), "l")),
            Expr::Some(inner) => self.emit_expr(inner, ns),
            Expr::Trust(inner) => self.emit_expr(inner, ns),

            Expr::Builtin { name, args } => match name.as_str() {
                "puts" => {
                    let (arg, _) = self.emit_expr(&args[0], ns)?;
                    self.emit(&format!("    call $puts(l {arg})"));
                    Ok(("0".into(), "w"))
                }
                "printf" => {
                    let fmt_tmp = if let Some(first) = args.first() {
                        match first {
                            Expr::StrLit(s) => {
                                let rewritten = rewrite_format_string(s);
                                let label = self.intern_string(&rewritten);
                                let t = self.fresh_tmp();
                                self.emit(&format!("    {t} =l copy ${label}"));
                                t
                            }
                            other => {
                                let (tmp, _) = self.emit_expr(other, ns)?;
                                tmp
                            }
                        }
                    } else {
                        return Err("@printf requires at least a format argument".into());
                    };
                    let mut variadic_args = Vec::new();
                    for a in args.iter().skip(1) {
                        let (tmp, ty) = self.emit_expr(a, ns)?;
                        let (promoted, pty) = self.promote_to_l(tmp, ty);
                        variadic_args.push(format!("{pty} {promoted}"));
                    }
                    if variadic_args.is_empty() {
                        self.emit(&format!("    call $printf(l {fmt_tmp})"));
                    } else {
                        self.emit(&format!(
                            "    call $printf(l {fmt_tmp}, ..., {})",
                            variadic_args.join(", ")
                        ));
                    }
                    Ok(("0".into(), "w"))
                }
                other => Err(format!("unknown builtin: @{other}")),
            },

            Expr::Field(_base, _field_name) => {
                let ptr = self.emit_field_ptr(expr, ns)?;
                let tmp = self.fresh_tmp();
                self.emit(&format!("    {tmp} =w loadw {ptr}"));
                Ok((tmp, "w"))
            }

            Expr::StructLit { fields } => {
                let size = fields.len() * 8;
                let ptr = self.fresh_tmp();
                self.emit(&format!("    {ptr} =l call $malloc(l {size})"));
                for (i, (_fname, fexpr)) in fields.iter().enumerate() {
                    let (val, val_ty) = self.emit_expr(fexpr, ns)?;
                    let offset = i * 8;
                    let field_ptr = self.fresh_tmp();
                    self.emit(&format!("    {field_ptr} =l add {ptr}, {offset}"));
                    if val_ty == "l" {
                        let w = self.fresh_tmp();
                        self.emit(&format!("    {w} =w copy {val}"));
                        self.emit(&format!("    storew {w}, {field_ptr}"));
                    } else {
                        self.emit(&format!("    storew {val}, {field_ptr}"));
                    }
                }
                Ok((ptr, "l"))
            }

            Expr::ArgsPack(exprs) => {
                if let Some(first) = exprs.first() {
                    self.emit_expr(first, ns)
                } else {
                    Ok(("0".into(), "l"))
                }
            }

            Expr::Call { callee, args } => {
                let fn_name = resolve_call_name(callee, ns)?;

                if fn_name == "put_any" {
                    let fmt_str = match args.first() {
                        Some(Expr::StrLit(s)) => s.clone(),
                        _ => return Err("put_any first argument must be a string literal".into()),
                    };
                    let rewritten = rewrite_format_string(&fmt_str);
                    let label = self.intern_string(&rewritten);
                    let fmt_tmp = self.fresh_tmp();
                    self.emit(&format!("    {fmt_tmp} =l copy ${label}"));
                    let mut variadic_args = Vec::new();
                    for a in args.iter().skip(1) {
                        if let Expr::ArgsPack(pack) = a {
                            for item in pack {
                                let (tmp, ty) = self.emit_expr(item, ns)?;
                                let (promoted, pty) = self.promote_to_l(tmp, ty);
                                variadic_args.push(format!("{pty} {promoted}"));
                            }
                        } else {
                            let (tmp, ty) = self.emit_expr(a, ns)?;
                            let (promoted, pty) = self.promote_to_l(tmp, ty);
                            variadic_args.push(format!("{pty} {promoted}"));
                        }
                    }
                    let result = self.fresh_tmp();
                    if variadic_args.is_empty() {
                        self.emit(&format!("    {result} =l call $printf(l {fmt_tmp})"));
                    } else {
                        self.emit(&format!(
                            "    {result} =l call $printf(l {fmt_tmp}, ..., {})",
                            variadic_args.join(", ")
                        ));
                    }
                    return Ok((result, "l"));
                }

                let mut arg_strs = Vec::new();
                for a in args.iter() {
                    if let Expr::ArgsPack(pack) = a {
                        for item in pack {
                            let (tmp, ty) = self.emit_expr(item, ns)?;
                            arg_strs.push(format!("{ty} {tmp}"));
                        }
                    } else {
                        let (tmp, ty) = self.emit_expr(a, ns)?;
                        arg_strs.push(format!("{ty} {tmp}"));
                    }
                }
                let result = self.fresh_tmp();
                self.emit(&format!(
                    "    {result} =l call ${fn_name}({args})",
                    args = arg_strs.join(", ")
                ));
                Ok((result, "l"))
            }

            Expr::BinOp { op, lhs, rhs } => {
                use crate::parser::BinOp::*;
                let (l, _) = self.emit_expr(lhs, ns)?;
                let (r, _) = self.emit_expr(rhs, ns)?;
                let tmp = self.fresh_tmp();
                let instr = match op {
                    Add => format!("{tmp} =w add {l}, {r}"),
                    Sub => format!("{tmp} =w sub {l}, {r}"),
                    Mul => format!("{tmp} =w mul {l}, {r}"),
                    Div => format!("{tmp} =w div {l}, {r}"),
                    Rem => format!("{tmp} =w rem {l}, {r}"),
                    Eq => format!("{tmp} =w ceqw {l}, {r}"),
                    Ne => format!("{tmp} =w cnew {l}, {r}"),
                    Lt => format!("{tmp} =w csltw {l}, {r}"),
                    Gt => format!("{tmp} =w csgtw {l}, {r}"),
                    Le => format!("{tmp} =w cslew {l}, {r}"),
                    Ge => format!("{tmp} =w csgew {l}, {r}"),
                    And => format!("{tmp} =w and {l}, {r}"),
                    Or => format!("{tmp} =w or {l}, {r}"),
                };
                self.emit(&format!("    {instr}"));
                Ok((tmp, "w"))
            }

            Expr::UnOp { op, expr } => {
                use crate::parser::UnOp::*;
                let (v, _) = self.emit_expr(expr, ns)?;
                let tmp = self.fresh_tmp();
                match op {
                    Not => self.emit(&format!("    {tmp} =w ceqw {v}, 0")),
                    Neg => self.emit(&format!("    {tmp} =w neg {v}")),
                }
                Ok((tmp, "w"))
            }
        }
    }
}

/// A flat map from dotted path (e.g. "std.io.print") → QBE function name
type Namespace = HashMap<String, String>;

fn build_namespace(resolved: &ResolvedModule) -> Namespace {
    let mut ns = HashMap::new();
    collect_ns(resolved, "", &mut ns);
    ns
}

fn build_trusted_set(resolved: &ResolvedModule) -> std::collections::HashSet<String> {
    let mut trusted = std::collections::HashSet::new();
    collect_trusted(resolved, "", &mut trusted);
    trusted
}

fn collect_trusted(
    module: &ResolvedModule,
    prefix: &str,
    trusted: &mut std::collections::HashSet<String>,
) {
    for item in &module.ast.items {
        if let Item::Fn(f) = item {
            if f.trusted {
                let key = if prefix.is_empty() {
                    f.name.clone()
                } else {
                    format!("{prefix}.{}", f.name)
                };
                trusted.insert(key);
            }
        }
    }
    for (import_name, child) in &module.imports {
        let child_prefix = if prefix.is_empty() {
            import_name.clone()
        } else {
            format!("{prefix}.{import_name}")
        };
        collect_trusted(child, &child_prefix, trusted);
    }
}

fn collect_ns(module: &ResolvedModule, prefix: &str, ns: &mut Namespace) {
    for item in &module.ast.items {
        match item {
            Item::Fn(f) if f.public => {
                let key = if prefix.is_empty() {
                    f.name.clone()
                } else {
                    format!("{prefix}.{}", f.name)
                };
                ns.insert(key, f.name.clone());
            }
            _ => {}
        }
    }

    for (import_name, child) in &module.imports {
        let child_prefix = if prefix.is_empty() {
            import_name.clone()
        } else {
            format!("{prefix}.{import_name}")
        };
        collect_ns(child, &child_prefix, ns);
    }
}

fn resolve_call_name(callee: &Expr, ns: &Namespace) -> Result<String, String> {
    let path = expr_to_path(callee);
    ns.get(&path)
        .cloned()
        .ok_or_else(|| format!("unknown function: {path}"))
}

fn expr_to_path(expr: &Expr) -> String {
    match expr {
        Expr::Ident(name) => name.clone(),
        Expr::Field(base, field) => format!("{}.{field}", expr_to_path(base)),
        _ => String::new(),
    }
}

/// Get the root identifier name from a (possibly nested) field access expression.
fn expr_root_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Ident(name) => Some(name.clone()),
        Expr::Field(base, _) => expr_root_name(base),
        _ => None,
    }
}

/// Translate Spectre format specifiers to printf specifiers.
/// {d} → %d, {s} → %s, {f} → %f, {x} → %x, etc.
fn rewrite_format_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            let mut spec = String::new();
            let mut closed = false;
            for inner in chars.by_ref() {
                if inner == '}' {
                    closed = true;
                    break;
                }
                spec.push(inner);
            }
            if closed && !spec.is_empty() {
                out.push('%');
                out.push_str(&spec);
            } else {
                out.push('{');
                out.push_str(&spec);
                if !closed { /* truncated, leave as-is */ }
            }
        } else {
            out.push(c);
        }
    }
    out
}
