use crate::cli::Platform;
use crate::module::ResolvedModule;
use crate::parser::{Expr, Field, FnDef, Item, Stmt, TypeExpr};
use std::collections::HashMap;

fn qbe_type(ty: &TypeExpr) -> &'static str {
    match ty {
        TypeExpr::Named(n) => match n.as_str() {
            "i32" | "u32" => "w",
            "i64" | "u64" | "usize" | "isize" => "l",
            "i8" | "u8" | "i16" | "u16" | "bool" => "w",
            "f32" => "s",
            "f64" => "d",
            "ptr" | "rawptr" => "l",
            _ => "l",
        },
        TypeExpr::Slice(_) => "l",
        TypeExpr::Ref(_) => "l",
        TypeExpr::Option(_) => "l",
        TypeExpr::FnPtr { .. } => "l",
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
    local_types: HashMap<String, &'static str>,
    local_mutability: HashMap<String, bool>,
    local_type_annotations: HashMap<String, String>,
    local_is_slot: std::collections::HashSet<String>,
    local_slot_is_l: std::collections::HashSet<String>,
    local_slot_is_d: std::collections::HashSet<String>,
    type_defs: HashMap<String, Vec<Field>>,
    union_defs: HashMap<String, Vec<TypeExpr>>,
    enum_defs: HashMap<String, Vec<String>>,
    trusted_fns: std::collections::HashSet<String>,
    current_fn: String,
    defer_stack: Vec<Vec<Stmt>>,
    current_loop_end: Option<String>,
    when_chain_end: Option<String>,
    test_fns: Vec<String>,
    current_file: String,
    module_consts: HashMap<String, (String, &'static str)>,
    type_aliases: HashMap<String, String>,
    fn_ret_types: HashMap<String, &'static str>,
    fn_param_types: HashMap<String, Vec<TypeExpr>>,
    platform: Platform,
    release: bool,
    current_fn_trusted: bool,
    in_trust_expr: bool,
    pub warnings: Vec<String>,
}

impl Codegen {
    pub fn new() -> Self {
        Self {
            out: String::new(),
            data: Vec::new(),
            str_counter: 0,
            tmp_counter: 0,
            locals: HashMap::new(),
            local_types: HashMap::new(),
            local_mutability: HashMap::new(),
            local_type_annotations: HashMap::new(),
            local_is_slot: std::collections::HashSet::new(),
            local_slot_is_l: std::collections::HashSet::new(),
            local_slot_is_d: std::collections::HashSet::new(),
            type_defs: HashMap::new(),
            union_defs: HashMap::new(),
            enum_defs: HashMap::new(),
            trusted_fns: std::collections::HashSet::new(),
            current_fn: String::new(),
            defer_stack: Vec::new(),
            current_loop_end: None,
            when_chain_end: None,
            test_fns: Vec::new(),
            current_file: String::new(),
            module_consts: HashMap::new(),
            type_aliases: HashMap::new(),
            fn_ret_types: HashMap::new(),
            fn_param_types: HashMap::new(),
            platform: Platform::current(),
            release: false,
            current_fn_trusted: false,
            in_trust_expr: false,
            warnings: Vec::new(),
        }
    }

    pub fn finish(mut self) -> String {
        let mut data_section = String::new();
        for (label, value) in &self.data {
            data_section.push_str(&format!("data ${label} = {{ b \"{value}\", b 0 }}\n"));
        }
        data_section.push_str("data $str_w_mode = { b \"w\", b 0 }\n");
        data_section.push_str("data $str_r_mode = { b \"r\", b 0 }\n");

        let stream_wrappers = concat!(
            "function l $sx_stdout() {\n@start\n",
            "    %r =l call $fdopen(w 1, l $str_w_mode)\n",
            "    ret %r\n}\n",
            "function l $sx_stderr() {\n@start\n",
            "    %r =l call $fdopen(w 2, l $str_w_mode)\n",
            "    ret %r\n}\n",
            "function l $sx_stdin() {\n@start\n",
            "    %r =l call $fdopen(w 0, l $str_r_mode)\n",
            "    ret %r\n}\n",
        );

        if !data_section.is_empty() {
            self.out.push('\n');
            self.out.push_str(&data_section);
        }
        self.out.push('\n');
        self.out.push_str(stream_wrappers);
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

    pub fn emit_module(
        &mut self,
        resolved: &ResolvedModule,
        test_mode: bool,
        release: bool,
    ) -> Result<(), String> {
        self.release = release;
        let ns = build_namespace(resolved);
        let trusted = build_trusted_set(resolved);
        self.trusted_fns = trusted;
        self.fn_ret_types = build_ret_types(resolved);
        self.fn_param_types = build_param_types(resolved);
        self.emit_module_recursive(resolved, &ns, test_mode, true)?;
        if test_mode {
            self.emit_test_main()?;
        }
        Ok(())
    }

    fn emit_module_recursive(
        &mut self,
        resolved: &ResolvedModule,
        ns: &Namespace,
        test_mode: bool,
        is_root: bool,
    ) -> Result<(), String> {
        for child in resolved.imports.values() {
            self.emit_module_recursive(child, ns, test_mode, false)?;
        }

        let prev_file = self.current_file.clone();
        self.current_file = resolved.filename.clone();

        for item in &resolved.ast.items {
            if let Item::TypeDef { name, fields, .. } = item {
                self.type_defs.insert(name.clone(), fields.clone());
            }
            if let Item::UnionDef { name, variants, .. } = item {
                self.union_defs.insert(name.clone(), variants.clone());
            }
            if let Item::EnumDef { name, variants, .. } = item {
                self.enum_defs.insert(name.clone(), variants.clone());
            }
        }

        self.module_consts.clear();
        for item in &resolved.ast.items {
            if let Item::Const { name, expr, .. } = item {
                let (val, ty) = match expr {
                    crate::parser::Expr::IntLit(n) => (n.to_string(), "l"),
                    crate::parser::Expr::FloatLit(f) => (format!("d_{f}"), "d"),
                    crate::parser::Expr::Bool(b) => (if *b { "1" } else { "0" }.to_string(), "w"),
                    crate::parser::Expr::UnOp {
                        op: crate::parser::UnOp::Neg,
                        expr,
                    } => match expr.as_ref() {
                        crate::parser::Expr::IntLit(n) => (format!("-{n}"), "l"),
                        crate::parser::Expr::FloatLit(f) => (format!("d_-{f}"), "d"),
                        _ => continue,
                    },
                    _ => continue,
                };
                self.module_consts.insert(name.clone(), (val, ty));
            }
        }

        let mut local_ns = ns.clone();
        for item in &resolved.ast.items {
            if let Item::Fn(f) = item {
                let local_key = match &f.namespace {
                    Some(type_name) => format!("{type_name}.{}", f.name),
                    None => f.name.clone(),
                };
                local_ns.insert(local_key, fn_qbe_name(f));
            }
            if let Item::ExternFn { name, symbol, .. } = item {
                local_ns.insert(name.clone(), symbol.clone());
            }
        }

        for item in &resolved.ast.items {
            match item {
                Item::Fn(f) => {
                    if test_mode && f.name == "main" {
                        continue;
                    }
                    self.emit_fn(f, &local_ns)?
                }
                Item::Test { body } if test_mode && is_root => {
                    self.emit_test_fn(body, &local_ns)?
                }
                Item::Use { .. }
                | Item::Const { .. }
                | Item::TypeDef { .. }
                | Item::UnionDef { .. }
                | Item::EnumDef { .. }
                | Item::ExternFn { .. }
                | Item::Link { .. }
                | Item::LinkWhen { .. }
                | Item::Test { .. } => {}
            }
        }

        self.current_file = prev_file;
        Ok(())
    }

    fn emit_fn(&mut self, f: &FnDef, ns: &Namespace) -> Result<(), String> {
        self.locals.clear();
        self.local_types.clear();
        self.local_mutability.clear();
        self.local_type_annotations.clear();
        self.local_is_slot.clear();
        self.local_slot_is_l.clear();
        self.local_slot_is_d.clear();
        self.defer_stack.clear();
        self.type_aliases.clear();
        self.when_chain_end = None;
        self.tmp_counter = 0;

        for (name, (val, ty)) in &self.module_consts.clone() {
            self.locals.insert(name.clone(), val.clone());
            self.local_types.insert(name.clone(), ty);
            self.local_mutability.insert(name.clone(), false);
        }

        let qbe_name = fn_qbe_name(f);
        self.current_fn = qbe_name.clone();
        self.current_fn_trusted = f.trusted;

        if !f.trusted {
            if let Some(builtin_name) = find_bare_builtin_in_stmts(&f.body) {
                return Err(format!(
                    "function '{}': builtin '@{}' called without 'trust' — \
                     either wrap the call with 'trust @{}(...)' or mark the function as unsafe with '!'",
                    qbe_name, builtin_name, builtin_name
                ));
            }
        }

        if !f.trusted && !self.release {
            let has_pre = f.body.iter().any(|s| matches!(s, Stmt::Pre(_)));
            let has_post = f.body.iter().any(|s| matches!(s, Stmt::Post(_)));
            if !all_trusted_stmts(&f.body) && !has_pre && !has_post {
                return Err(format!(
                    "pure function '{}' must have at least one 'pre' or 'post' contract block, or consist entirely of 'trust' statements",
                    qbe_name
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
                let qty = qbe_type(ty);
                self.locals.insert(name.clone(), tmp.clone());
                self.local_types.insert(name.clone(), qty);
                if let TypeExpr::Named(type_name) = ty {
                    self.local_type_annotations
                        .insert(name.clone(), type_name.clone());
                }
                format!("{qty} {tmp}")
            })
            .collect();

        self.emit(&format!(
            "{export}function {ret_ty}${name}({params}) {{",
            name = qbe_name,
            params = params.join(", ")
        ));
        self.emit("@start");

        for stmt in &f.body {
            self.emit_stmt(stmt, ns, &f.ret)?;
        }

        if matches!(f.ret, TypeExpr::Void) {
            self.emit_defers(ns, &f.ret)?;
            self.emit("    ret");
        }

        self.emit("}");
        self.emit("");
        Ok(())
    }

    fn emit_test_fn(&mut self, body: &[Stmt], ns: &Namespace) -> Result<(), String> {
        static mut TEST_COUNTER: usize = 0;
        let test_id = unsafe {
            TEST_COUNTER += 1;
            TEST_COUNTER
        };

        self.locals.clear();
        self.local_types.clear();
        self.local_mutability.clear();
        self.local_type_annotations.clear();
        self.local_is_slot.clear();
        self.local_slot_is_l.clear();
        self.local_slot_is_d.clear();
        self.defer_stack.clear();
        self.type_aliases.clear();
        self.when_chain_end = None;
        self.tmp_counter = 0;
        self.current_fn = format!("test_{}", test_id);
        self.current_fn_trusted = true;

        for (name, (val, ty)) in &self.module_consts.clone() {
            self.locals.insert(name.clone(), val.clone());
            self.local_types.insert(name.clone(), ty);
            self.local_mutability.insert(name.clone(), false);
        }

        self.emit(&format!("export function w $test_{}() {{", test_id));
        self.emit("@start");

        for stmt in body {
            self.emit_stmt(stmt, ns, &TypeExpr::Void)?;
        }

        self.emit("    ret 0");
        self.emit("}");
        self.emit("");
        self.test_fns.push(format!("test_{}", test_id));
        Ok(())
    }

    fn emit_test_main(&mut self) -> Result<(), String> {
        let fns = self.test_fns.clone();
        self.emit("export function w $main() {");
        self.emit("@start");
        for name in &fns {
            self.emit(&format!("    call ${name}()"));
        }
        let ok_lbl = self.intern_string("all tests passed\n");
        let ok_tmp = self.fresh_tmp();
        self.emit(&format!("    {ok_tmp} =l copy ${ok_lbl}"));
        self.emit(&format!("    call $printf(l {ok_tmp})"));
        self.emit("    ret 0");
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
                let path = expr_to_path(expr);
                if !path.is_empty() && is_namespace_prefix(&path, ns) {
                    let expanded = expand_alias_path(&path, &self.type_aliases);
                    self.type_aliases.insert(name.clone(), expanded);
                    return Ok(());
                }

                let (tmp, qty) = self.emit_expr(expr, ns)?;
                if *mutable {
                    let slot_qty = if let Some(TypeExpr::Named(type_name)) = ty {
                        qbe_type(&TypeExpr::Named(type_name.clone()))
                    } else {
                        qty
                    };
                    let slot = self.fresh_tmp();
                    self.emit(&format!("    {slot} =l alloc8 8"));
                    if slot_qty == "d" {
                        self.emit(&format!("    stored {tmp}, {slot}"));
                    } else if slot_qty == "l" {
                        let (tmp_l, _) = self.promote_to_l(tmp, qty);
                        self.emit(&format!("    storel {tmp_l}, {slot}"));
                    } else {
                        self.emit(&format!("    storew {tmp}, {slot}"));
                    }
                    self.locals.insert(name.clone(), slot.clone());
                    self.local_types.insert(name.clone(), slot_qty);
                    self.local_mutability.insert(name.clone(), true);
                    self.local_is_slot.insert(name.clone());
                    if slot_qty == "l" {
                        self.local_slot_is_l.insert(name.clone());
                    } else if slot_qty == "d" {
                        self.local_slot_is_d.insert(name.clone());
                    }
                } else {
                    self.locals.insert(name.clone(), tmp);
                    self.local_types.insert(name.clone(), qty);
                    self.local_mutability.insert(name.clone(), *mutable);
                }
                if let Some(TypeExpr::Named(type_name)) = ty {
                    self.local_type_annotations
                        .insert(name.clone(), type_name.clone());
                }
            }

            Stmt::Assign { target, value } => {
                if let Some(root) = expr_root_name(target) {
                    let is_mut = self.local_mutability.get(&root).copied().unwrap_or(false);
                    if !is_mut {
                        return Err(format!("cannot assign to immutable binding '{root}'"));
                    }
                }
                if let Expr::Ident(name) = target {
                    let (val_tmp, val_qty) = self.emit_expr(value, ns)?;
                    if self.local_is_slot.contains(name) {
                        let slot = self
                            .locals
                            .get(name)
                            .cloned()
                            .ok_or_else(|| format!("undefined variable: {name}"))?;
                        let is_d_slot = self.local_slot_is_d.contains(name);
                        let is_l_slot = self.local_slot_is_l.contains(name);
                        if is_d_slot {
                            self.emit(&format!("    stored {val_tmp}, {slot}"));
                        } else if is_l_slot {
                            let (val_l, _) = self.promote_to_l(val_tmp, val_qty);
                            self.emit(&format!("    storel {val_l}, {slot}"));
                        } else {
                            self.emit(&format!("    storew {val_tmp}, {slot}"));
                        }
                    } else {
                        self.locals.insert(name.clone(), val_tmp);
                        self.local_types.insert(name.clone(), val_qty);
                    }
                    return Ok(());
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
                let store_val = if val_ty == "w" {
                    let (promoted, _) = self.promote_to_l(val_tmp, val_ty);
                    promoted
                } else {
                    val_tmp
                };
                let ptr = self.emit_field_ptr(target, ns)?;
                self.emit(&format!("    storel {store_val}, {ptr}"));
            }

            Stmt::Return(None) => {
                self.emit_defers(ns, ret_ty)?;
                self.emit("    ret");
            }
            Stmt::Return(Some(expr)) => {
                let (tmp, _) = self.emit_expr(expr, ns)?;
                self.emit_defers(ns, ret_ty)?;
                self.emit(&format!("    ret {tmp}"));
            }
            Stmt::Expr(expr) => {
                self.emit_expr(expr, ns)?;
            }
            Stmt::Pre(contracts) => {
                if !self.release {
                    for c in contracts {
                        let (cond, _) = self.emit_expr(&c.expr, ns)?;
                        let ok_lbl = format!("@pre_ok_{}", self.tmp_counter);
                        let fail_lbl = format!("@pre_fail_{}", self.tmp_counter);
                        self.tmp_counter += 1;
                        self.emit(&format!("    jnz {cond}, {ok_lbl}, {fail_lbl}"));
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
            }
            Stmt::Post(contracts) => {
                if !self.release {
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
            }
            Stmt::If {
                cond,
                then,
                elif_,
                else_,
            } => {
                let id = self.tmp_counter;
                self.tmp_counter += 1;

                let end_lbl = format!("@if_end_{id}");
                let then_lbl = format!("@if_then_{id}");
                let first_else_lbl = if !elif_.is_empty() {
                    format!("@elif_cond_0_{id}")
                } else if else_.is_some() {
                    format!("@if_else_{id}")
                } else {
                    end_lbl.clone()
                };

                let elif_labels: Vec<(String, String, String)> = (0..elif_.len())
                    .map(|i| {
                        let cond_lbl = format!("@elif_cond_{i}_{id}");
                        let body_lbl = format!("@elif_body_{i}_{id}");
                        let next_lbl = if i + 1 < elif_.len() {
                            format!("@elif_cond_{}_{id}", i + 1)
                        } else if else_.is_some() {
                            format!("@if_else_{id}")
                        } else {
                            end_lbl.clone()
                        };
                        (cond_lbl, body_lbl, next_lbl)
                    })
                    .collect();

                let (cond_tmp, _) = self.emit_expr(cond, ns)?;
                self.emit(&format!("    jnz {cond_tmp}, {then_lbl}, {first_else_lbl}"));
                self.emit(&format!("{then_lbl}"));
                for s in then {
                    self.emit_stmt(s, ns, ret_ty)?;
                }
                if !block_is_terminated(then) {
                    self.emit(&format!("    jmp {end_lbl}"));
                }

                for (i, (elif_cond, elif_body)) in elif_.iter().enumerate() {
                    let (cond_lbl, body_lbl, next_lbl) = &elif_labels[i];
                    self.emit(&format!("{cond_lbl}"));
                    let (ec, _) = self.emit_expr(elif_cond, ns)?;
                    self.emit(&format!("    jnz {ec}, {body_lbl}, {next_lbl}"));
                    self.emit(&format!("{body_lbl}"));
                    for s in elif_body {
                        self.emit_stmt(s, ns, ret_ty)?;
                    }
                    if !block_is_terminated(elif_body) {
                        self.emit(&format!("    jmp {end_lbl}"));
                    }
                }

                if let Some(else_stmts) = else_ {
                    self.emit(&format!("@if_else_{id}"));
                    for s in else_stmts {
                        self.emit_stmt(s, ns, ret_ty)?;
                    }
                    if !block_is_terminated(else_stmts) {
                        self.emit(&format!("    jmp {end_lbl}"));
                    }
                }

                self.emit(&format!("{end_lbl}"));
            }
            Stmt::For {
                init,
                cond,
                post,
                body,
            } => {
                let loop_lbl = format!("@for_loop_{}", self.tmp_counter);
                let body_lbl = format!("@for_body_{}", self.tmp_counter);
                let end_lbl = format!("@for_end_{}", self.tmp_counter);
                self.tmp_counter += 1;

                if let Some((var, init_expr)) = init {
                    let (tmp, qty) = self.emit_expr(init_expr, ns)?;
                    let slot = self.fresh_tmp();
                    self.emit(&format!("    {slot} =l alloc8 8"));
                    let (tmp_l, _) = self.promote_to_l(tmp, qty);
                    self.emit(&format!("    storel {tmp_l}, {slot}"));
                    self.locals.insert(var.clone(), slot.clone());
                    self.local_mutability.insert(var.clone(), true);
                    self.local_is_slot.insert(var.clone());
                    self.local_slot_is_l.insert(var.clone());
                }

                self.emit(&format!("    jmp {loop_lbl}"));
                self.emit(&format!("{loop_lbl}"));

                if let Some(cond_expr) = cond {
                    let (ct, _) = self.emit_expr(cond_expr, ns)?;
                    self.emit(&format!("    jnz {ct}, {body_lbl}, {end_lbl}"));
                } else {
                    self.emit(&format!("    jmp {body_lbl}"));
                }

                self.emit(&format!("{body_lbl}"));
                let prev_loop_end = self.current_loop_end.replace(end_lbl.clone());
                for s in body {
                    self.emit_stmt(s, ns, ret_ty)?;
                }
                self.current_loop_end = prev_loop_end;

                if !block_is_terminated(body) {
                    if let Some(post_stmt) = post {
                        self.emit_stmt(post_stmt, ns, ret_ty)?;
                    }
                    self.emit(&format!("    jmp {loop_lbl}"));
                }
                self.emit(&format!("{end_lbl}"));
            }
            Stmt::Increment(var) => {
                let slot = self
                    .locals
                    .get(var)
                    .cloned()
                    .ok_or_else(|| format!("undefined variable: {var}"))?;
                let is_l_slot = self.local_slot_is_l.contains(var);
                let cur = self.fresh_tmp();
                let inc = self.fresh_tmp();
                if is_l_slot {
                    self.emit(&format!("    {cur} =l loadl {slot}"));
                    self.emit(&format!("    {inc} =l add {cur}, 1"));
                    self.emit(&format!("    storel {inc}, {slot}"));
                } else {
                    self.emit(&format!("    {cur} =w loadw {slot}"));
                    self.emit(&format!("    {inc} =w add {cur}, 1"));
                    self.emit(&format!("    storew {inc}, {slot}"));
                }
            }
            Stmt::Defer(body) => {
                self.defer_stack.push(body.clone());
            }
            Stmt::Break => {
                let end_lbl = self
                    .current_loop_end
                    .clone()
                    .ok_or_else(|| "break used outside of loop".to_string())?;
                self.emit(&format!("    jmp {end_lbl}"));
            }
            Stmt::When { platform, body } => {
                let matches = Platform::from_str(platform)
                    .map(|p| p == self.platform)
                    .unwrap_or(false);
                if matches {
                    for s in body {
                        self.emit_stmt(s, ns, ret_ty)?;
                    }
                }
            }
            Stmt::WhenIs { expr, ty, body } => {
                let chain_end = if let Some(lbl) = &self.when_chain_end {
                    lbl.clone()
                } else {
                    let lbl = format!("@when_end_{}", self.tmp_counter);
                    self.tmp_counter += 1;
                    self.when_chain_end = Some(lbl.clone());
                    lbl
                };

                let tag_index = self.resolve_union_tag(expr, ty)?;
                let body_lbl = format!("@when_body_{}", self.tmp_counter);
                let skip_lbl = format!("@when_skip_{}", self.tmp_counter);
                self.tmp_counter += 1;

                let (union_ptr, _) = self.emit_expr(expr, ns)?;
                let tag_tmp = self.fresh_tmp();
                self.emit(&format!("    {tag_tmp} =w loadw {union_ptr}"));
                let cond_tmp = self.fresh_tmp();
                self.emit(&format!("    {cond_tmp} =w ceqw {tag_tmp}, {tag_index}"));
                self.emit(&format!("    jnz {cond_tmp}, {body_lbl}, {skip_lbl}"));
                self.emit(&format!("{body_lbl}"));
                for s in body {
                    self.emit_stmt(s, ns, ret_ty)?;
                }
                if !block_is_terminated(body) {
                    self.emit(&format!("    jmp {chain_end}"));
                }
                self.emit(&format!("{skip_lbl}"));
            }
            Stmt::Otherwise { body } => {
                for s in body {
                    self.emit_stmt(s, ns, ret_ty)?;
                }
                if let Some(lbl) = self.when_chain_end.take() {
                    self.emit(&format!("{lbl}"));
                }
            }
            Stmt::Assert(expr, line) => {
                let (cond, _) = self.emit_expr(expr, ns)?;
                let pass_lbl = format!("@assert_pass_{}", self.tmp_counter);
                let fail_lbl = format!("@assert_fail_{}", self.tmp_counter);
                self.tmp_counter += 1;
                self.emit(&format!("    jnz {cond}, {pass_lbl}, {fail_lbl}"));
                self.emit(&format!("{fail_lbl}"));
                let msg = format!("{}:{}: assertion failed\n", self.current_file, line);
                let msg_lbl = self.intern_string(&msg);
                self.emit(&format!("    %stderr =l call $fdopen(w 2, l $str_w_mode)"));
                self.emit(&format!("    call $fprintf(l %stderr, l ${msg_lbl})"));
                self.emit(&format!("    call $fflush(l %stderr)"));
                self.emit("    call $abort()");
                self.emit(&format!("{pass_lbl}"));
            }
            Stmt::Match {
                expr,
                some_binding,
                some_body,
                none_body,
            } => {
                let (val_tmp, _) = self.emit_expr(expr, ns)?;
                let some_lbl = format!("@match_some_{}", self.tmp_counter);
                let none_lbl = format!("@match_none_{}", self.tmp_counter);
                let end_lbl = format!("@match_end_{}", self.tmp_counter);

                self.tmp_counter += 1;

                let cond_tmp = self.fresh_tmp();

                self.emit(&format!("    {cond_tmp} =w cnel {val_tmp}, 0"));
                self.emit(&format!("    jnz {cond_tmp}, {some_lbl}, {none_lbl}"));

                self.emit(&format!("{some_lbl}"));
                self.locals.insert(some_binding.clone(), val_tmp.clone());
                for s in some_body {
                    self.emit_stmt(s, ns, ret_ty)?;
                }
                if !block_is_terminated(some_body) {
                    self.emit(&format!("    jmp {end_lbl}"));
                }

                self.emit(&format!("{none_lbl}"));
                for s in none_body {
                    self.emit_stmt(s, ns, ret_ty)?;
                }
                if !block_is_terminated(none_body) {
                    self.emit(&format!("    jmp {end_lbl}"));
                }

                self.emit(&format!("{end_lbl}"));
            }
        }
        Ok(())
    }

    /// Emit all deferred blocks in LIFO order (does not pop — safe to call multiple times).
    fn emit_defers(&mut self, ns: &Namespace, ret_ty: &TypeExpr) -> Result<(), String> {
        let defers = self.defer_stack.clone();
        for body in defers.iter().rev() {
            for s in body {
                self.emit_stmt(s, ns, ret_ty)?;
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

    /// Resolve the tag index for `ty` within the union type of `expr`.
    /// Returns the 0-based variant index.
    fn resolve_union_tag(&self, expr: &Expr, ty: &TypeExpr) -> Result<usize, String> {
        let union_name = self.infer_struct_type_name(expr)?;
        let variants = self
            .union_defs
            .get(&union_name)
            .ok_or_else(|| format!("'{union_name}' is not a union type"))?;
        variants
            .iter()
            .position(|v| type_expr_matches(v, ty))
            .ok_or_else(|| format!("type is not a variant of union '{union_name}'"))
    }

    /// Promote a w value to l via sign-extension (needed for variadic call args).
    fn promote_to_l(&mut self, tmp: String, ty: &'static str) -> (String, &'static str) {
        if ty == "d" || ty == "l" {
            return (tmp, ty);
        }
        let ext = self.fresh_tmp();
        let is_literal = tmp.starts_with(|c: char| c.is_ascii_digit() || c == '-');
        if is_literal {
            self.emit(&format!("    {ext} =l copy {tmp}"));
        } else {
            self.emit(&format!("    {ext} =l extsw {tmp}"));
        }
        (ext, "l")
    }

    fn emit_expr(&mut self, expr: &Expr, ns: &Namespace) -> Result<(String, &'static str), String> {
        match expr {
            Expr::IntLit(n) => Ok((n.to_string(), "w")),

            Expr::FloatLit(f) => Ok((format!("d_{f}"), "d")),

            Expr::StrLit(s) => {
                let label = self.intern_string(s);
                let tmp = self.fresh_tmp();
                self.emit(&format!("    {tmp} =l copy ${label}"));
                Ok((tmp, "l"))
            }

            Expr::Ident(name) => {
                let slot_or_tmp =
                    self.locals.get(name).cloned().ok_or_else(|| {
                        format!("{}: undefined variable: {name}", self.current_file)
                    })?;
                if self.local_is_slot.contains(name) {
                    let tmp = self.fresh_tmp();
                    let is_d_slot = self.local_slot_is_d.contains(name);
                    let is_l_slot = self.local_slot_is_l.contains(name);
                    if is_d_slot {
                        self.emit(&format!("    {tmp} =d loadd {slot_or_tmp}"));
                        Ok((tmp, "d"))
                    } else if is_l_slot {
                        self.emit(&format!("    {tmp} =l loadl {slot_or_tmp}"));
                        Ok((tmp, "l"))
                    } else {
                        self.emit(&format!("    {tmp} =w loadw {slot_or_tmp}"));
                        Ok((tmp, "w"))
                    }
                } else {
                    let qty = self.local_types.get(name).copied().unwrap_or("l");
                    Ok((slot_or_tmp, qty))
                }
            }

            Expr::Bool(b) => Ok((if *b { "1".into() } else { "0".into() }, "w")),
            Expr::None => Ok(("0".into(), "l")),
            Expr::Some(inner) => {
                let (tmp, ty) = self.emit_expr(inner, ns)?;
                Ok(self.promote_to_l(tmp, ty))
            }
            Expr::Trust(inner) => {
                if self.current_fn_trusted {
                    self.warnings.push(format!(
                        "{}: redundant 'trust' in impure function '{}' — \
                         the function is already marked '!' so 'trust' has no effect",
                        self.current_file, self.current_fn
                    ));
                }
                self.in_trust_expr = true;
                let result = self.emit_expr(inner, ns);
                self.in_trust_expr = false;
                result
            }

            Expr::Builtin { name, args } => match name.as_str() {
                "puts" => {
                    let (arg, _) = self.emit_expr(&args[0], ns)?;
                    self.emit(&format!("    call $puts(l {arg})"));
                    Ok(("0".into(), "w"))
                }
                "alloc" => {
                    let (size, size_ty) = self.emit_expr(&args[0], ns)?;
                    let (size_l, _) = self.promote_to_l(size, size_ty);
                    let tmp = self.fresh_tmp();
                    self.emit(&format!("    {tmp} =l call $malloc(l {size_l})"));
                    Ok((tmp, "l"))
                }
                "realloc" => {
                    let (ptr, _) = self.emit_expr(&args[0], ns)?;
                    let (size, size_ty) = self.emit_expr(&args[1], ns)?;
                    let (size_l, _) = self.promote_to_l(size, size_ty);
                    let tmp = self.fresh_tmp();
                    self.emit(&format!("    {tmp} =l call $realloc(l {ptr}, l {size_l})"));
                    Ok((tmp, "l"))
                }
                "free" => {
                    let (ptr, _) = self.emit_expr(&args[0], ns)?;
                    self.emit(&format!("    call $free(l {ptr})"));
                    Ok(("0".into(), "w"))
                }
                "memset" => {
                    let (ptr, _) = self.emit_expr(&args[0], ns)?;
                    let (val, val_ty) = self.emit_expr(&args[1], ns)?;
                    let (size, size_ty) = self.emit_expr(&args[2], ns)?;
                    let (val_w, _) = if val_ty == "l" {
                        let w = self.fresh_tmp();
                        self.emit(&format!("    {w} =w copy {val}"));
                        (w, "w")
                    } else {
                        (val, val_ty)
                    };
                    let (size_l, _) = self.promote_to_l(size, size_ty);
                    self.emit(&format!("    call $memset(l {ptr}, w {val_w}, l {size_l})"));
                    Ok(("0".into(), "w"))
                }
                "ptradd" => {
                    let (ptr, _) = self.emit_expr(&args[0], ns)?;
                    let (off, off_ty) = self.emit_expr(&args[1], ns)?;
                    let (off_l, _) = self.promote_to_l(off, off_ty);
                    let tmp = self.fresh_tmp();
                    self.emit(&format!("    {tmp} =l add {ptr}, {off_l}"));
                    Ok((tmp, "l"))
                }
                "load" => {
                    let (ptr, _) = self.emit_expr(&args[0], ns)?;
                    let tmp = self.fresh_tmp();
                    self.emit(&format!("    {tmp} =l loadl {ptr}"));
                    Ok((tmp, "l"))
                }
                "store" => {
                    let (ptr, _) = self.emit_expr(&args[0], ns)?;
                    let (val, val_ty) = self.emit_expr(&args[1], ns)?;
                    let (val_l, _) = self.promote_to_l(val, val_ty);
                    self.emit(&format!("    storel {val_l}, {ptr}"));
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
                "getchar" => {
                    let tmp = self.fresh_tmp();
                    self.emit(&format!("    {tmp} =w call $getchar()"));
                    Ok((tmp, "w"))
                }
                "fgets" => {
                    let (buf, _) = self.emit_expr(&args[0], ns)?;
                    let (size, size_ty) = self.emit_expr(&args[1], ns)?;
                    let (stream, _) = self.emit_expr(&args[2], ns)?;
                    let (size_w, _) = if size_ty == "l" {
                        let w = self.fresh_tmp();
                        self.emit(&format!("    {w} =w copy {size}"));
                        (w, "w")
                    } else {
                        (size, size_ty)
                    };
                    let tmp = self.fresh_tmp();
                    self.emit(&format!(
                        "    {tmp} =l call $fgets(l {buf}, w {size_w}, l {stream})"
                    ));
                    Ok((tmp, "l"))
                }
                "fputs" => {
                    let (s, _) = self.emit_expr(&args[0], ns)?;
                    let (stream, _) = self.emit_expr(&args[1], ns)?;
                    let tmp = self.fresh_tmp();
                    self.emit(&format!("    {tmp} =w call $fputs(l {s}, l {stream})"));
                    Ok((tmp, "w"))
                }
                "fprintf" => {
                    if args.len() < 2 {
                        return Err("@fprintf requires stream and format arguments".into());
                    }
                    let (stream, _) = self.emit_expr(&args[0], ns)?;
                    let fmt_tmp = match &args[1] {
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
                    };
                    let mut variadic_args = Vec::new();
                    for a in args.iter().skip(2) {
                        let (tmp, ty) = self.emit_expr(a, ns)?;
                        let (promoted, pty) = self.promote_to_l(tmp, ty);
                        variadic_args.push(format!("{pty} {promoted}"));
                    }
                    let result = self.fresh_tmp();
                    if variadic_args.is_empty() {
                        self.emit(&format!(
                            "    {result} =w call $fprintf(l {stream}, l {fmt_tmp})"
                        ));
                    } else {
                        self.emit(&format!(
                            "    {result} =w call $fprintf(l {stream}, l {fmt_tmp}, ..., {})",
                            variadic_args.join(", ")
                        ));
                    }
                    Ok((result, "w"))
                }
                "fflush" => {
                    let (stream, _) = self.emit_expr(&args[0], ns)?;
                    let tmp = self.fresh_tmp();
                    self.emit(&format!("    {tmp} =w call $fflush(l {stream})"));
                    Ok((tmp, "w"))
                }
                "dprintf" => {
                    if args.len() < 2 {
                        return Err("@dprintf requires fd and format arguments".into());
                    }
                    let (fd, fd_ty) = self.emit_expr(&args[0], ns)?;
                    let (fd_w, _) = if fd_ty == "l" {
                        let w = self.fresh_tmp();
                        self.emit(&format!("    {w} =w copy {fd}"));
                        (w, "w")
                    } else {
                        (fd, fd_ty)
                    };
                    let fmt_tmp = match &args[1] {
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
                    };
                    let mut variadic_args = Vec::new();
                    for a in args.iter().skip(2) {
                        let (tmp, ty) = self.emit_expr(a, ns)?;
                        let (promoted, pty) = self.promote_to_l(tmp, ty);
                        variadic_args.push(format!("{pty} {promoted}"));
                    }
                    let result = self.fresh_tmp();
                    if variadic_args.is_empty() {
                        self.emit(&format!(
                            "    {result} =w call $dprintf(w {fd_w}, l {fmt_tmp})"
                        ));
                    } else {
                        self.emit(&format!(
                            "    {result} =w call $dprintf(w {fd_w}, l {fmt_tmp}, ..., {})",
                            variadic_args.join(", ")
                        ));
                    }
                    Ok((result, "w"))
                }
                "stdout" => {
                    let tmp = self.fresh_tmp();
                    self.emit(&format!("    {tmp} =l call $sx_stdout()"));
                    Ok((tmp, "l"))
                }
                "stderr" => {
                    let tmp = self.fresh_tmp();
                    self.emit(&format!("    {tmp} =l call $sx_stderr()"));
                    Ok((tmp, "l"))
                }
                "stdin" => {
                    let tmp = self.fresh_tmp();
                    self.emit(&format!("    {tmp} =l call $sx_stdin()"));
                    Ok((tmp, "l"))
                }
                "arc4random_buf" => {
                    let (buf, _) = self.emit_expr(&args[0], ns)?;
                    let (len, len_ty) = self.emit_expr(&args[1], ns)?;
                    let (len_l, _) = self.promote_to_l(len, len_ty);
                    self.emit(&format!("    call $arc4random_buf(l {buf}, l {len_l})"));
                    Ok(("0".into(), "w"))
                }
                "arc4random_uniform" => {
                    let (bound, bound_ty) = self.emit_expr(&args[0], ns)?;
                    let (bound_w, _) = if bound_ty == "l" {
                        let w = self.fresh_tmp();
                        self.emit(&format!("    {w} =w copy {bound}"));
                        (w, "w")
                    } else {
                        (bound, bound_ty)
                    };
                    let tmp = self.fresh_tmp();
                    self.emit(&format!(
                        "    {tmp} =w call $arc4random_uniform(w {bound_w})"
                    ));
                    Ok((tmp, "w"))
                }
                "snprintf" => {
                    if args.len() < 3 {
                        return Err("@snprintf requires buf, size, and format arguments".into());
                    }
                    let (buf, _) = self.emit_expr(&args[0], ns)?;
                    let (size, size_ty) = self.emit_expr(&args[1], ns)?;
                    let (size_l, _) = self.promote_to_l(size, size_ty);
                    let fmt_tmp = match &args[2] {
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
                    };
                    let mut variadic_args = Vec::new();
                    for a in args.iter().skip(3) {
                        let (tmp, ty) = self.emit_expr(a, ns)?;
                        let (promoted, pty) = self.promote_to_l(tmp, ty);
                        variadic_args.push(format!("{pty} {promoted}"));
                    }
                    let result = self.fresh_tmp();
                    if variadic_args.is_empty() {
                        self.emit(&format!(
                            "    {result} =w call $snprintf(l {buf}, l {size_l}, l {fmt_tmp})"
                        ));
                    } else {
                        self.emit(&format!(
                            "    {result} =w call $snprintf(l {buf}, l {size_l}, l {fmt_tmp}, ..., {})",
                            variadic_args.join(", ")
                        ));
                    }
                    Ok((result, "w"))
                }
                "gettimeofday" => {
                    let (tv, _) = self.emit_expr(&args[0], ns)?;
                    let tz = if args.len() > 1 {
                        let (t, _) = self.emit_expr(&args[1], ns)?;
                        t
                    } else {
                        "0".into()
                    };
                    let tmp = self.fresh_tmp();
                    self.emit(&format!("    {tmp} =w call $gettimeofday(l {tv}, l {tz})"));
                    Ok((tmp, "w"))
                }
                "localtime" => {
                    let (timep, _) = self.emit_expr(&args[0], ns)?;
                    let tmp = self.fresh_tmp();
                    self.emit(&format!("    {tmp} =l call $localtime(l {timep})"));
                    Ok((tmp, "l"))
                }
                other => Err(format!("unknown builtin: @{other}")),
            },

            Expr::Field(_base, _field_name) => {
                if let Expr::Ident(enum_name) = _base.as_ref() {
                    if let Some(variants) = self.enum_defs.get(enum_name.as_str()).cloned() {
                        let idx =
                            variants
                                .iter()
                                .position(|v| v == _field_name)
                                .ok_or_else(|| {
                                    format!("enum '{enum_name}' has no variant '{_field_name}'")
                                })?;
                        return Ok((idx.to_string(), "w"));
                    }
                }
                let ptr = self.emit_field_ptr(expr, ns)?;
                let tmp = self.fresh_tmp();
                self.emit(&format!("    {tmp} =l loadl {ptr}"));
                Ok((tmp, "l"))
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
                        self.emit(&format!("    storel {val}, {field_ptr}"));
                    } else {
                        let (ext, _) = self.promote_to_l(val, val_ty);
                        self.emit(&format!("    storel {ext}, {field_ptr}"));
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

            Expr::Call { callee, args, line } => {
                let callee_path = expr_to_path(callee);
                let is_local_fnptr = !callee_path.is_empty()
                    && !callee_path.contains('.')
                    && self.locals.contains_key(&callee_path)
                    && !ns.contains_key(&callee_path);

                if is_local_fnptr {
                    let (ptr, _) = self.emit_expr(callee, ns)?;
                    self.in_trust_expr = false;
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
                        "    {result} =l call {ptr}({args})",
                        args = arg_strs.join(", ")
                    ));
                    return Ok((result, "l"));
                }

                let fn_name = resolve_call_name(callee, ns, &self.type_aliases.clone())
                    .map_err(|e| format!("{}:{}: {}", self.current_file, line, e))?;

                if !self.current_fn_trusted && !self.in_trust_expr {
                    if self.trusted_fns.contains(&fn_name) {
                        return Err(format!(
                            "{}:{}: pure function '{}' calls impure function '{}' — \
                             wrap the call with 'trust' or mark the caller as impure with '!'",
                            self.current_file, line, self.current_fn, fn_name
                        ));
                    }
                }
                self.in_trust_expr = false;

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
                for (i, a) in args.iter().enumerate() {
                    if let Expr::ArgsPack(pack) = a {
                        for item in pack {
                            let (tmp, ty) = self.emit_expr(item, ns)?;
                            arg_strs.push(format!("{ty} {tmp}"));
                        }
                    } else {
                        let (tmp, ty) = self.emit_expr(a, ns)?;
                        let param_types = self.fn_param_types.get(&fn_name).cloned();
                        let wrapped = if let Some(ref ptypes) = param_types {
                            if let Some(TypeExpr::Named(union_name)) = ptypes.get(i) {
                                if let Some(variants) = self.union_defs.get(union_name).cloned() {
                                    let arg_type_name = match a {
                                        Expr::Ident(n) => {
                                            self.local_type_annotations.get(n).cloned()
                                        }
                                        _ => None,
                                    };
                                    let tag = arg_type_name.as_deref().and_then(|atn| {
                                        variants.iter().position(|v| {
                                            matches!(v, TypeExpr::Named(n) if n == atn)
                                                || matches!(v, TypeExpr::Ref(_) if atn == "ref")
                                        })
                                    });
                                    if let Some(tag_idx) = tag {
                                        let ptr = self.fresh_tmp();
                                        self.emit(&format!("    {ptr} =l call $malloc(l 16)"));
                                        self.emit(&format!("    storew {tag_idx}, {ptr}"));
                                        let val_ptr = self.fresh_tmp();
                                        self.emit(&format!("    {val_ptr} =l add {ptr}, 8"));
                                        let (val_l, _) = self.promote_to_l(tmp.clone(), ty);
                                        self.emit(&format!("    storel {val_l}, {val_ptr}"));
                                        Some(format!("l {ptr}"))
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        arg_strs.push(wrapped.unwrap_or_else(|| format!("{ty} {tmp}")));
                    }
                }
                let result = self.fresh_tmp();
                let ret_ty = self
                    .fn_ret_types
                    .get(fn_name.as_str())
                    .copied()
                    .unwrap_or("l");
                self.emit(&format!(
                    "    {result} ={ret_ty} call ${fn_name}({args})",
                    args = arg_strs.join(", ")
                ));
                Ok((result, ret_ty))
            }

            Expr::BinOp { op, lhs, rhs } => {
                use crate::parser::BinOp::*;
                let (l, l_ty) = self.emit_expr(lhs, ns)?;
                let (r, r_ty) = self.emit_expr(rhs, ns)?;
                let tmp = self.fresh_tmp();
                if l_ty == "d" || r_ty == "d" {
                    let instr = match op {
                        Add => format!("{tmp} =d add {l}, {r}"),
                        Sub => format!("{tmp} =d sub {l}, {r}"),
                        Mul => format!("{tmp} =d mul {l}, {r}"),
                        Div => format!("{tmp} =d div {l}, {r}"),
                        Rem => format!("{tmp} =d rem {l}, {r}"),
                        Eq => format!("{tmp} =w ceqd {l}, {r}"),
                        Ne => format!("{tmp} =w cned {l}, {r}"),
                        Lt => format!("{tmp} =w cltd {l}, {r}"),
                        Gt => format!("{tmp} =w cgtd {l}, {r}"),
                        Le => format!("{tmp} =w cled {l}, {r}"),
                        Ge => format!("{tmp} =w cged {l}, {r}"),
                        And => format!("{tmp} =d and {l}, {r}"),
                        Or => format!("{tmp} =d or {l}, {r}"),
                        _ => return Err("bitwise/shift ops not supported on f64".into()),
                    };
                    self.emit(&format!("    {instr}"));
                    let result_ty = match op {
                        Eq | Ne | Lt | Gt | Le | Ge => "w",
                        _ => "d",
                    };
                    return Ok((tmp, result_ty));
                }
                let wide = l_ty == "l" || r_ty == "l";
                let (l, r) = if wide {
                    let (l, _) = self.promote_to_l(l, l_ty);
                    let (r, _) = self.promote_to_l(r, r_ty);
                    (l, r)
                } else {
                    (l, r)
                };
                let instr = if wide {
                    match op {
                        Add => format!("{tmp} =l add {l}, {r}"),
                        Sub => format!("{tmp} =l sub {l}, {r}"),
                        Mul => format!("{tmp} =l mul {l}, {r}"),
                        Div => format!("{tmp} =l div {l}, {r}"),
                        Rem => format!("{tmp} =l rem {l}, {r}"),
                        Eq => format!("{tmp} =w ceql {l}, {r}"),
                        Ne => format!("{tmp} =w cnel {l}, {r}"),
                        Lt => format!("{tmp} =w csltl {l}, {r}"),
                        Gt => format!("{tmp} =w csgtl {l}, {r}"),
                        Le => format!("{tmp} =w cslel {l}, {r}"),
                        Ge => format!("{tmp} =w csgel {l}, {r}"),
                        And => format!("{tmp} =l and {l}, {r}"),
                        Or => format!("{tmp} =l or {l}, {r}"),
                        BitAnd => format!("{tmp} =l and {l}, {r}"),
                        BitOr => format!("{tmp} =l or {l}, {r}"),
                        Shl => format!("{tmp} =l shl {l}, {r}"),
                        Shr => format!("{tmp} =l shr {l}, {r}"),
                    }
                } else {
                    match op {
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
                        BitAnd => format!("{tmp} =w and {l}, {r}"),
                        BitOr => format!("{tmp} =w or {l}, {r}"),
                        Shl => format!("{tmp} =w shl {l}, {r}"),
                        Shr => format!("{tmp} =w shr {l}, {r}"),
                    }
                };
                self.emit(&format!("    {instr}"));
                let result_ty = match op {
                    Eq | Ne | Lt | Gt | Le | Ge => "w",
                    _ => {
                        if wide {
                            "l"
                        } else {
                            "w"
                        }
                    }
                };
                Ok((tmp, result_ty))
            }

            Expr::UnOp { op, expr } => {
                use crate::parser::UnOp::*;
                let (v, v_ty) = self.emit_expr(expr, ns)?;
                let tmp = self.fresh_tmp();
                match op {
                    Not => {
                        self.emit(&format!("    {tmp} =w ceqw {v}, 0"));
                        Ok((tmp, "w"))
                    }
                    Neg => {
                        if v_ty == "w" {
                            self.emit(&format!("    {tmp} =w neg {v}"));
                            let (promoted, _) = self.promote_to_l(tmp, "w");
                            Ok((promoted, "l"))
                        } else {
                            self.emit(&format!("    {tmp} ={v_ty} neg {v}"));
                            Ok((tmp, v_ty))
                        }
                    }
                    BitwiseNot => {
                        let (v_l, _) = self.promote_to_l(v, v_ty);
                        self.emit(&format!("    {tmp} =l xor {v_l}, -1"));
                        Ok((tmp, "l"))
                    }
                }
            }

            Expr::Cast { expr, ty } => {
                let (v, v_ty) = self.emit_expr(expr, ns)?;
                let target_ty = qbe_type(ty);
                let tmp = self.fresh_tmp();
                match (v_ty, target_ty) {
                    // same type — no-op copy
                    (a, b) if a == b => {
                        self.emit(&format!("    {tmp} ={b} copy {v}"));
                        Ok((tmp, target_ty))
                    }
                    // f64 -> i64/usize
                    ("d", "l") => {
                        self.emit(&format!("    {tmp} =l dtosi {v}"));
                        Ok((tmp, "l"))
                    }
                    // i64/usize -> f64
                    ("l", "d") => {
                        self.emit(&format!("    {tmp} =d sltof {v}"));
                        Ok((tmp, "d"))
                    }
                    // i32 -> f64
                    ("w", "d") => {
                        let (v_l, _) = self.promote_to_l(v, "w");
                        self.emit(&format!("    {tmp} =d sltof {v_l}"));
                        Ok((tmp, "d"))
                    }
                    // f64 -> i32
                    ("d", "w") => {
                        self.emit(&format!("    {tmp} =w dtosi {v}"));
                        Ok((tmp, "w"))
                    }
                    // i32 -> i64
                    ("w", "l") => {
                        let (promoted, _) = self.promote_to_l(v, "w");
                        self.emit(&format!("    {tmp} =l copy {promoted}"));
                        Ok((tmp, "l"))
                    }
                    // i64 -> i32
                    ("l", "w") => {
                        self.emit(&format!("    {tmp} =w copy {v}"));
                        Ok((tmp, "w"))
                    }
                    (from, to) => Err(format!("unsupported cast from {from} to {to}")),
                }
            }

            Expr::Addr(inner) => {
                let tmp = self.fresh_tmp();
                match inner.as_ref() {
                    Expr::Ident(name) => {
                        if let Some(qbe_name) = ns.get(name) {
                            self.emit(&format!("    {tmp} =l copy ${qbe_name}"));
                            return Ok((tmp, "l"));
                        }
                        if self.local_is_slot.contains(name) {
                            let slot = self.locals.get(name).cloned()
                                .ok_or_else(|| format!("undefined variable: {name}"))?;
                            self.emit(&format!("    {tmp} =l copy {slot}"));
                            return Ok((tmp, "l"));
                        }
                        Err(format!("cannot take address of immutable binding '{name}' — declare it as 'mut' or use a function name"))
                    }
                    other => {
                        let path = expr_to_path(other);
                        let expanded = expand_alias_path(&path, &self.type_aliases.clone());
                        if let Some(qbe_name) = ns.get(&expanded) {
                            self.emit(&format!("    {tmp} =l copy ${qbe_name}"));
                            Ok((tmp, "l"))
                        } else {
                            Err(format!("addr(): '{path}' is not a known function"))
                        }
                    }
                }
            }

            Expr::Deref(inner) => {
                let (ptr, _) = self.emit_expr(inner, ns)?;
                let tmp = self.fresh_tmp();
                self.emit(&format!("    {tmp} =l loadl {ptr}"));
                Ok((tmp, "l"))
            }
        }
    }
}

/// Returns true if the last statement in a block is a terminator (return),
/// meaning no fall-through jump is needed.
fn block_is_terminated(stmts: &[Stmt]) -> bool {
    matches!(stmts.last(), Some(Stmt::Return(_)) | Some(Stmt::Break))
}

/// Recursively checks whether a block consists entirely of "trusted" operations —
/// i.e. no raw untrusted calls or assignments that would require contracts.
/// Control flow constructs (if, for, match, defer) are transparent: we recurse into them.
fn all_trusted_stmts(stmts: &[Stmt]) -> bool {
    stmts.iter().all(|s| match s {
        Stmt::Expr(Expr::Trust(_)) => true,
        Stmt::Expr(Expr::Call { callee, .. }) => !expr_to_path(callee).is_empty(),
        Stmt::Expr(Expr::Builtin { .. }) => true,
        Stmt::Pre(_) | Stmt::Post(_) => true,
        Stmt::Val { .. } | Stmt::Return(_) | Stmt::Break | Stmt::Increment(_) => true,
        Stmt::Assert(..) => true,
        Stmt::Assign { .. } => false,
        Stmt::Defer(body) => all_trusted_stmts(body),
        Stmt::If {
            then, elif_, else_, ..
        } => {
            all_trusted_stmts(then)
                && elif_.iter().all(|(_, b)| all_trusted_stmts(b))
                && else_.as_deref().map_or(true, all_trusted_stmts)
        }
        Stmt::For { body, .. } => all_trusted_stmts(body),
        Stmt::Match {
            some_body,
            none_body,
            ..
        } => all_trusted_stmts(some_body) && all_trusted_stmts(none_body),
        Stmt::When { body, .. } => all_trusted_stmts(body),
        Stmt::WhenIs { body, .. } => all_trusted_stmts(body),
        Stmt::Otherwise { body } => all_trusted_stmts(body),
        Stmt::Expr(_) => false,
    })
}

type Namespace = HashMap<String, String>;

/// Returns the name of the first bare (non-trusted) builtin found in a statement list,
/// or None if all builtins are properly wrapped with `trust`.
fn find_bare_builtin_in_stmts(stmts: &[Stmt]) -> Option<String> {
    stmts.iter().find_map(find_bare_builtin_in_stmt)
}

fn find_bare_builtin_in_stmt(stmt: &Stmt) -> Option<String> {
    match stmt {
        Stmt::Expr(expr) => find_bare_builtin_in_expr(expr),
        Stmt::Val { expr, .. } => find_bare_builtin_in_expr(expr),
        Stmt::Assign { value, .. } => find_bare_builtin_in_expr(value),
        Stmt::Return(Some(expr)) => find_bare_builtin_in_expr(expr),
        Stmt::Return(None) => None,
        Stmt::Pre(contracts) => contracts
            .iter()
            .find_map(|c| find_bare_builtin_in_expr(&c.expr)),
        Stmt::Post(contracts) => contracts
            .iter()
            .find_map(|c| find_bare_builtin_in_expr(&c.expr)),
        Stmt::Assert(expr, _) => find_bare_builtin_in_expr(expr),
        Stmt::Defer(body) => find_bare_builtin_in_stmts(body),
        Stmt::If {
            cond,
            then,
            elif_,
            else_,
        } => find_bare_builtin_in_expr(cond)
            .or_else(|| find_bare_builtin_in_stmts(then))
            .or_else(|| {
                elif_.iter().find_map(|(e, b)| {
                    find_bare_builtin_in_expr(e).or_else(|| find_bare_builtin_in_stmts(b))
                })
            })
            .or_else(|| else_.as_deref().and_then(find_bare_builtin_in_stmts)),
        Stmt::For {
            init,
            cond,
            post,
            body,
        } => init
            .as_ref()
            .and_then(|(_, e)| find_bare_builtin_in_expr(e))
            .or_else(|| cond.as_ref().and_then(find_bare_builtin_in_expr))
            .or_else(|| post.as_deref().and_then(find_bare_builtin_in_stmt))
            .or_else(|| find_bare_builtin_in_stmts(body)),
        Stmt::Match {
            expr,
            some_body,
            none_body,
            ..
        } => find_bare_builtin_in_expr(expr)
            .or_else(|| find_bare_builtin_in_stmts(some_body))
            .or_else(|| find_bare_builtin_in_stmts(none_body)),
        Stmt::When { body, .. } => find_bare_builtin_in_stmts(body),
        Stmt::WhenIs { expr, body, .. } => {
            find_bare_builtin_in_expr(expr).or_else(|| find_bare_builtin_in_stmts(body))
        }
        Stmt::Otherwise { body } => find_bare_builtin_in_stmts(body),
        Stmt::Increment(_) | Stmt::Break => None,
    }
}

fn find_bare_builtin_in_expr(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Trust(_) => None,
        Expr::Builtin { name, .. } => Some(name.clone()),
        Expr::Call { callee, args, .. } => find_bare_builtin_in_expr(callee)
            .or_else(|| args.iter().find_map(find_bare_builtin_in_expr)),
        Expr::BinOp { lhs, rhs, .. } => {
            find_bare_builtin_in_expr(lhs).or_else(|| find_bare_builtin_in_expr(rhs))
        }
        Expr::UnOp { expr, .. }
        | Expr::Cast { expr, .. }
        | Expr::Some(expr)
        | Expr::Addr(expr)
        | Expr::Deref(expr) => find_bare_builtin_in_expr(expr),
        Expr::Field(base, _) => find_bare_builtin_in_expr(base),
        Expr::StructLit { fields } => fields
            .iter()
            .find_map(|(_, e)| find_bare_builtin_in_expr(e)),
        Expr::ArgsPack(exprs) => exprs.iter().find_map(find_bare_builtin_in_expr),
        Expr::IntLit(_)
        | Expr::FloatLit(_)
        | Expr::StrLit(_)
        | Expr::Ident(_)
        | Expr::Bool(_)
        | Expr::None => None,
    }
}

fn build_namespace(resolved: &ResolvedModule) -> Namespace {
    let mut ns = HashMap::new();
    collect_ns(resolved, "", &mut ns);
    ns
}

fn build_ret_types(resolved: &ResolvedModule) -> HashMap<String, &'static str> {
    let mut map = HashMap::new();
    collect_ret_types(resolved, &mut map);
    map
}

fn collect_ret_types(module: &ResolvedModule, map: &mut HashMap<String, &'static str>) {
    for item in &module.ast.items {
        match item {
            Item::Fn(f) => { map.insert(fn_qbe_name(f), qbe_type(&f.ret)); }
            Item::ExternFn { symbol, ret, .. } => { map.insert(symbol.clone(), qbe_type(ret)); }
            _ => {}
        }
    }
    for child in module.imports.values() {
        collect_ret_types(child, map);
    }
}

fn build_param_types(resolved: &ResolvedModule) -> HashMap<String, Vec<TypeExpr>> {
    let mut map = HashMap::new();
    collect_param_types(resolved, &mut map);
    map
}

fn collect_param_types(module: &ResolvedModule, map: &mut HashMap<String, Vec<TypeExpr>>) {
    for item in &module.ast.items {
        match item {
            Item::Fn(f) => {
                map.insert(fn_qbe_name(f), f.params.iter().map(|(_, ty)| ty.clone()).collect());
            }
            Item::ExternFn { symbol, params, .. } => {
                map.insert(symbol.clone(), params.iter().map(|(_, ty)| ty.clone()).collect());
            }
            _ => {}
        }
    }
    for child in module.imports.values() {
        collect_param_types(child, map);
    }
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
        match item {
            Item::Fn(f) if f.trusted => {
                let local_key = match &f.namespace {
                    Some(type_name) => format!("{type_name}.{}", f.name),
                    None => f.name.clone(),
                };
                let key = if prefix.is_empty() { local_key } else { format!("{prefix}.{local_key}") };
                trusted.insert(key);
            }
            Item::ExternFn { name, symbol, .. } => {
                let key = if prefix.is_empty() { name.clone() } else { format!("{prefix}.{name}") };
                trusted.insert(key);
                trusted.insert(symbol.clone());
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
        collect_trusted(child, &child_prefix, trusted);
    }
}

/// Compute the QBE-level symbol name for a function.
/// Namespaced methods are mangled: `SomeType__method_name`
fn fn_qbe_name(f: &FnDef) -> String {
    match &f.namespace {
        Some(type_name) => format!("{}__{}", type_name, f.name),
        None => f.name.clone(),
    }
}

fn collect_ns(module: &ResolvedModule, prefix: &str, ns: &mut Namespace) {
    for item in &module.ast.items {
        match item {
            Item::Fn(f) => {
                if f.namespace.is_some() && !f.public {
                    continue;
                }
                let qbe = fn_qbe_name(f);
                let local_key = match &f.namespace {
                    Some(type_name) => format!("{type_name}.{}", f.name),
                    None => f.name.clone(),
                };
                let key = if prefix.is_empty() {
                    local_key
                } else {
                    format!("{prefix}.{local_key}")
                };
                ns.insert(key, qbe);
            }
            Item::ExternFn { name, symbol, public, .. } => {
                if !public {
                    continue;
                }
                let key = if prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{prefix}.{name}")
                };
                ns.insert(key, symbol.clone());
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

fn resolve_call_name(
    callee: &Expr,
    ns: &Namespace,
    aliases: &HashMap<String, String>,
) -> Result<String, String> {
    let path = expr_to_path(callee);
    let expanded = expand_alias_path(&path, aliases);
    ns.get(&expanded)
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

/// Expand the leading segment of a dotted path through the alias map.
/// e.g. "Stack.new" with alias Stack="std.allocators.stack.Stack" → "std.allocators.stack.Stack.new"
fn expand_alias_path(path: &str, aliases: &HashMap<String, String>) -> String {
    let (head, tail) = match path.find('.') {
        Some(i) => (&path[..i], Some(&path[i + 1..])),
        None => (path, None),
    };
    if let Some(expanded_head) = aliases.get(head) {
        match tail {
            Some(rest) => format!("{expanded_head}.{rest}"),
            None => expanded_head.clone(),
        }
    } else {
        path.to_string()
    }
}

/// Returns true if `path` (or any prefix of it) matches a key in the namespace,
/// meaning it refers to a namespace segment rather than a concrete runtime value.
fn is_namespace_prefix(path: &str, ns: &Namespace) -> bool {
    ns.keys()
        .any(|k| k == path || k.starts_with(&format!("{path}.")))
}

/// Get the root identifier name from a (possibly nested) field access expression.
fn expr_root_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Ident(name) => Some(name.clone()),
        Expr::Field(base, _) => expr_root_name(base),
        _ => None,
    }
}

/// Check if two TypeExpr values refer to the same type (for union variant matching).
fn type_expr_matches(a: &TypeExpr, b: &TypeExpr) -> bool {
    match (a, b) {
        (TypeExpr::Named(x), TypeExpr::Named(y)) => x == y,
        (TypeExpr::Ref(x), TypeExpr::Ref(y)) => type_expr_matches(x, y),
        (TypeExpr::Slice(x), TypeExpr::Slice(y)) => type_expr_matches(x, y),
        (TypeExpr::Option(x), TypeExpr::Option(y)) => type_expr_matches(x, y),
        (TypeExpr::Void, TypeExpr::Void) => true,
        _ => false,
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
