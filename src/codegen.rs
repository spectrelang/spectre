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
        TypeExpr::FixedArray(_, _) => "a",
        TypeExpr::Ref(_) => "l",
        TypeExpr::Option(_) => "l",
        TypeExpr::List(_) => "l",
        TypeExpr::Result(_, _) => "l",
        TypeExpr::FnPtr { .. } => "l",
        TypeExpr::Mut(inner) => qbe_type(inner),
        TypeExpr::Void => "",
        TypeExpr::Untyped => "l",
    }
}

/// Convert a TypeExpr to the annotation string used for union variant tag matching.
pub fn type_to_annotation_string(ty: &TypeExpr) -> String {
    match ty {
        TypeExpr::Named(n) => n.clone(),
        TypeExpr::Ref(inner) => format!("ref {}", type_to_annotation_string(inner)),
        TypeExpr::Option(inner) => format!("option[{}]", type_to_annotation_string(inner)),
        TypeExpr::List(inner) => format!("list[{}]", type_to_annotation_string(inner)),
        TypeExpr::Result(ok, err) => format!(
            "result[{}, {}]",
            type_to_annotation_string(ok),
            type_to_annotation_string(err)
        ),
        _ => String::new(),
    }
}

/// Compute the byte size of a type, used for fixed array layout.
fn type_byte_size(ty: &TypeExpr) -> u64 {
    match ty {
        TypeExpr::Named(n) => match n.as_str() {
            "i8" | "u8" | "char" => 1,
            "i16" | "u16" => 2,
            "i32" | "u32" => 4,
            "i64" | "u64" | "usize" | "isize" | "ptr" | "rawptr" => 8,
            "f32" => 4,
            "f64" => 8,
            _ => 8,
        },
        TypeExpr::Slice(_) => 16,
        TypeExpr::FixedArray(count, elem_ty) => {
            let elem_size = type_byte_size(elem_ty);
            count * elem_size
        }
        TypeExpr::Ref(_) => 8,
        TypeExpr::Option(_) => 8,
        TypeExpr::List(_) => 24,
        TypeExpr::Result(_, _) => 16,
        TypeExpr::FnPtr { .. } => 8,
        TypeExpr::Mut(inner) => type_byte_size(inner),
        TypeExpr::Void => 0,
        TypeExpr::Untyped => 8,
    }
}

/// Compute the alignment requirement of a type in bytes.
fn type_alignment(ty: &TypeExpr) -> u64 {
    match ty {
        TypeExpr::Named(n) => match n.as_str() {
            "i8" | "u8" | "char" => 1,
            "i16" | "u16" => 2,
            "i32" | "u32" | "f32" => 4,
            "i64" | "u64" | "usize" | "isize" | "f64" | "ptr" | "rawptr" => 8,
            _ => 8,
        },
        TypeExpr::Slice(_) => 8,
        TypeExpr::FixedArray(_, elem_ty) => type_alignment(elem_ty),
        TypeExpr::Ref(_) => 8,
        TypeExpr::Option(_) => 8,
        TypeExpr::List(_) => 8,
        TypeExpr::Result(_, _) => 8,
        TypeExpr::FnPtr { .. } => 8,
        TypeExpr::Mut(inner) => type_alignment(inner),
        TypeExpr::Void => 1,
        TypeExpr::Untyped => 8,
    }
}

/// Round up to nearest multiple of `align`.
fn align_to(n: u64, align: u64) -> u64 {
    if align == 0 {
        return n;
    }
    (n + align - 1) / align * align
}

/// Round up to nearest multiple of 8 for alignment.
fn align8(n: u64) -> u64 {
    align_to(n, 8)
}

/// Returns true if a parameter type is a `ref` (possibly wrapped in other modifiers),
/// meaning the parameter can be assigned through inside the function body.
fn is_ref_param_type(ty: &TypeExpr) -> bool {
    matches!(ty, TypeExpr::Ref(_) | TypeExpr::Mut(_))
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
    extern_type_defs: HashMap<String, Vec<Field>>,
    union_defs: HashMap<String, Vec<TypeExpr>>,
    enum_defs: HashMap<String, Vec<String>>,
    fixed_array_types: HashMap<String, String>,
    fixed_array_counter: usize,
    trusted_fns: std::collections::HashSet<String>,
    current_fn: String,
    defer_stack: Vec<Vec<Stmt>>,
    current_loop_end: Option<String>,
    current_loop_continue: Option<String>,
    test_fns: Vec<String>,
    current_file: String,
    module_consts: HashMap<String, (String, &'static str)>,
    cross_module_consts: HashMap<String, (String, &'static str)>,
    module_aliases: HashMap<String, String>,
    type_aliases: HashMap<String, String>,
    fn_ret_types: HashMap<String, &'static str>,
    fn_ret_type_exprs: HashMap<String, TypeExpr>,
    fn_param_types: HashMap<String, Vec<TypeExpr>>,
    variadic_fns: HashMap<String, usize>,
    result_void_ok: std::collections::HashSet<String>,
    fn_ptr_consts: HashMap<String, String>,
    platform: Platform,
    release: bool,
    current_fn_trusted: bool,
    in_trust_expr: bool,
    current_fn_ret: TypeExpr,
    current_module_prefix: String,
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
            extern_type_defs: HashMap::new(),
            union_defs: HashMap::new(),
            enum_defs: HashMap::new(),
            fixed_array_types: HashMap::new(),
            fixed_array_counter: 0,
            trusted_fns: std::collections::HashSet::new(),
            current_fn: String::new(),
            defer_stack: Vec::new(),
            current_loop_end: None,
            current_loop_continue: None,
            test_fns: Vec::new(),
            current_file: String::new(),
            module_consts: HashMap::new(),
            cross_module_consts: HashMap::new(),
            module_aliases: HashMap::new(),
            type_aliases: HashMap::new(),
            fn_ret_types: HashMap::new(),
            fn_ret_type_exprs: HashMap::new(),
            fn_param_types: HashMap::new(),
            variadic_fns: HashMap::new(),
            result_void_ok: std::collections::HashSet::new(),
            fn_ptr_consts: HashMap::new(),
            platform: Platform::current(),
            release: false,
            current_fn_trusted: false,
            in_trust_expr: false,
            current_fn_ret: TypeExpr::Void,
            current_module_prefix: String::new(),
            warnings: Vec::new(),
        }
    }

    pub fn finish(mut self) -> String {
        let mut type_section = String::new();
        for (key, qbe_label) in &self.fixed_array_types {
            let parts: Vec<&str> = key.split('_').collect();
            let count: u64 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
            let elem_size: u64 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(8);
            let total_bytes = count * elem_size;
            let aligned_bytes = align8(total_bytes);
            let num_slots = (aligned_bytes / 8) as usize;

            let members: Vec<&str> = (0..num_slots).map(|_| "l").collect();
            let type_def = members.join(" ");
            type_section.push_str(&format!("type {} = {{ {} }}\n", qbe_label, type_def));
        }

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

        let args_globals = concat!(
            "data $sx_argc_store = { l 0 }\n",
            "data $sx_argv_store = { l 0 }\n",
        );
        let get_args_fn = concat!(
            "function l $sx_get_args() {\n@start\n",
            "    %argc =l loadl $sx_argc_store\n",
            "    %argv =l loadl $sx_argv_store\n",
            // allocate List header (24 bytes)
            "    %hdr =l call $malloc(l 24)\n",
            // allocate backing buffer: argc * 8 bytes
            "    %cap =l mul %argc, 8\n",
            "    %buf =l call $malloc(l %cap)\n",
            // store ptr, len=0, cap=argc
            "    storel %buf, %hdr\n",
            "    %len_slot =l add %hdr, 8\n",
            "    storel 0, %len_slot\n",
            "    %cap_slot =l add %hdr, 16\n",
            "    storel %argc, %cap_slot\n",
            // loop: push each argv[i] into the list
            "    %i =l copy 0\n",
            "@args_loop\n",
            "    %done =l csgtl %i, %argc\n",
            "    jnz %done, @args_done, @args_body\n",
            "@args_body\n",
            "    %off =l mul %i, 8\n",
            "    %slot =l add %argv, %off\n",
            "    %str =l loadl %slot\n",
            // push: get current len, store str at buf[len], increment len
            "    %cur_len =l loadl %len_slot\n",
            "    %boff =l mul %cur_len, 8\n",
            "    %bslot =l add %buf, %boff\n",
            "    storel %str, %bslot\n",
            "    %new_len =l add %cur_len, 1\n",
            "    storel %new_len, %len_slot\n",
            "    %i =l add %i, 1\n",
            "    jmp @args_loop\n",
            "@args_done\n",
            "    ret %hdr\n",
            "}\n",
        );

        if !type_section.is_empty() {
            self.out.push_str(&type_section);
            self.out.push('\n');
        }
        if !data_section.is_empty() {
            self.out.push('\n');
            self.out.push_str(&data_section);
        }
        self.out.push('\n');
        self.out.push_str(stream_wrappers);
        self.out.push('\n');
        self.out.push_str(args_globals);
        self.out.push('\n');
        self.out.push_str(get_args_fn);
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

    /// Get or create a QBE type label for a fixed array type.
    /// Returns the QBE type reference (e.g., ":fa_0") and the aligned byte size.
    #[allow(dead_code)]
    fn get_or_create_fixed_array_type(&mut self, count: u64, elem_ty: &TypeExpr) -> (String, u64) {
        let elem_size = type_byte_size(elem_ty);
        let total_bytes = count * elem_size;
        let aligned_bytes = align8(total_bytes);
        let key = format!("fa_{count}_{elem_size}");
        if let Some(label) = self.fixed_array_types.get(&key) {
            return (label.clone(), aligned_bytes);
        }

        let label = format!(":fa_{}", self.fixed_array_counter);
        self.fixed_array_counter += 1;

        let num_slots = (aligned_bytes / 8) as usize;
        let members: Vec<&str> = (0..num_slots).map(|_| "l").collect();
        members.join(" ");
        self.fixed_array_types.insert(key, label.clone());

        (label, aligned_bytes)
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
        self.fn_ret_type_exprs = build_ret_type_exprs(resolved);
        self.fn_param_types = build_param_types(resolved);
        self.variadic_fns = build_variadic_set(resolved);
        self.result_void_ok = build_result_void_ok(resolved);
        self.cross_module_consts = build_cross_module_consts(resolved);
        self.emit_module_recursive(resolved, &ns, test_mode, true, "")?;
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
        module_prefix: &str,
    ) -> Result<(), String> {
        if !module_prefix.is_empty() {
            println!("--- emit_module_recursive: prefix='{}' file='{}'", module_prefix, resolved.filename);
        }
        for (import_name, child) in &resolved.imports {
            let child_prefix = if module_prefix.is_empty() {
                import_name.clone()
            } else {
                format!("{module_prefix}__{import_name}")
            };
            self.emit_module_recursive(child, ns, test_mode, false, &child_prefix)?;
        }

        let prev_file = self.current_file.clone();
        let prev_prefix = self.current_module_prefix.clone();
        self.current_file = resolved.filename.clone();
        self.current_module_prefix = module_prefix.to_string();

        let items = self.flatten_items(&resolved.ast.items);

        for item in &items {
            if let Item::TypeDef { name, fields, .. } = item {
                self.type_defs.insert(name.clone(), fields.clone());
            }
            if let Item::ExternTypeDef { name, fields, .. } = item {
                self.extern_type_defs.insert(name.clone(), fields.clone());
            }
            if let Item::UnionDef { name, variants, .. } = item {
                self.union_defs.insert(name.clone(), variants.clone());
            }
            if let Item::EnumDef { name, variants, .. } = item {
                self.enum_defs.insert(name.clone(), variants.clone());
            }
        }

        self.module_consts.clear();
        for item in &items {
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

        self.module_aliases.clear();
        let ns_prefix = module_prefix.replace("__", ".");
        for (import_name, _) in &resolved.imports {
            let expanded = if ns_prefix.is_empty() {
                import_name.clone()
            } else {
                format!("{ns_prefix}.{import_name}")
            };
            self.module_aliases.insert(import_name.clone(), expanded.clone());
            println!("  alias: {} -> {}", import_name, expanded);
        }

        for item in &items {
            if let Item::Const { name, expr, .. } = item {
                let path = expr_to_path(expr);
                if !path.is_empty() {
                    let expanded = expand_alias_path(&path, &self.module_aliases);
                    if let Some(qbe_name) = ns.get(&expanded) {
                        self.fn_ptr_consts.insert(name.clone(), qbe_name.clone());
                    } else if is_namespace_prefix(&path, ns) {
                        self.module_aliases.insert(name.clone(), expanded);
                    }
                }
            }
        }

        let mut local_ns = ns.clone();
        for item in &items {
            if let Item::Fn(f) = item {
                let local_key = match &f.namespace {
                    Some(type_name) => format!("{type_name}.{}", f.name),
                    None => f.name.clone(),
                };
                local_ns.insert(local_key, fn_qbe_name_prefixed(f, module_prefix));
            }
            if let Item::ExternFn { name, symbol, .. } = item {
                local_ns.insert(name.clone(), symbol.clone());
            }
        }

        for item in &items {
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
                | Item::ExternTypeDef { .. }
                | Item::UnionDef { .. }
                | Item::EnumDef { .. }
                | Item::ExternFn { .. }
                | Item::Link { .. }
                | Item::LinkWhen { .. }
                | Item::WhenItems { .. }
                | Item::Test { .. } => {}
            }
        }

        self.current_file = prev_file;
        self.current_module_prefix = prev_prefix;
        Ok(())
    }

    fn flatten_items<'a>(&self, items: &'a [Item]) -> Vec<&'a Item> {
        let mut result = Vec::new();
        for item in items {
            match item {
                Item::WhenItems {
                    platform,
                    items: block_items,
                } => {
                    if self.platform.matches_name(platform) {
                        for bi in block_items {
                            result.push(bi);
                        }
                    }
                }
                other => result.push(other),
            }
        }
        result
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
        self.type_aliases = self.module_aliases.clone();
        self.tmp_counter = 0;

        for (name, (val, ty)) in &self.module_consts.clone() {
            self.locals.insert(name.clone(), val.clone());
            self.local_types.insert(name.clone(), ty);
            self.local_mutability.insert(name.clone(), false);
        }

        let qbe_name = {
            let base = fn_qbe_name(f);
            if self.current_module_prefix.is_empty() || base == "main" {
                base
            } else {
                format!("{}__{}", self.current_module_prefix, base)
            }
        };
        self.current_fn = qbe_name.clone();
        self.current_fn_trusted = f.trusted;
        self.current_fn_ret = f.ret.clone();

        if RESERVED_SYMBOLS.contains(&qbe_name.as_str()) {
            return Err(format!(
                "{}: function '{}' collides with a reserved symbol used by the runtime — rename it",
                self.current_file, qbe_name
            ));
        }

        if !f.trusted {
            if let Some(builtin_name) = find_bare_builtin_in_stmts(&f.body) {
                return Err(format!(
                    "function '{}': builtin '@{}' called without 'trust' — \
                     either wrap the call with 'trust @{}(...)' or mark the function as unsafe with '!'",
                    qbe_name, builtin_name, builtin_name
                ));
            }
            if find_bare_deref_assign_in_stmts(&f.body) {
                return Err(format!(
                    "function '{}': 'deref(...) = ...' used without 'trust' — \
                     wrap the assignment with 'trust deref(...) = ...' or mark the function as unsafe with '!'",
                    qbe_name
                ));
            }
        }

        if !f.trusted && !self.release {
            let has_pre = f
                .body
                .iter()
                .any(|s| matches!(s, Stmt::Pre(_) | Stmt::GuardedPre(_)));
            let has_post = f
                .body
                .iter()
                .any(|s| matches!(s, Stmt::Post(_) | Stmt::GuardedPost(_)));
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
        let ret_ty = if qbe_name == "main" {
            "w ".to_string()
        } else {
            ret_ty
        };

        let params: Vec<String> = f
            .params
            .iter()
            .map(|(name, ty)| {
                let tmp = format!("%{name}");
                let qty = qbe_type(ty);
                self.locals.insert(name.clone(), tmp.clone());
                self.local_types.insert(name.clone(), qty);
                let is_mutable = is_ref_param_type(ty);
                self.local_mutability.insert(name.clone(), is_mutable);
                let inner_ty = if let TypeExpr::Mut(inner) = ty { inner.as_ref() } else { ty };
                let ann = type_to_annotation_string(inner_ty);
                if !ann.is_empty() {
                    self.local_type_annotations.insert(name.clone(), ann);
                }
                format!("{qty} {tmp}")
            })
            .collect();

        self.emit(&format!(
            "{export}function {ret_ty}${name}({params}) {{",
            name = qbe_name,
            params = if qbe_name == "main" {
                "w %argc, l %argv".to_string()
            } else {
                params.join(", ")
            }
        ));
        self.emit("@start");

        if qbe_name == "main" {
            self.emit("    %sx_argc_val =l extsw %argc");
            self.emit("    storel %sx_argc_val, $sx_argc_store");
            self.emit("    storel %argv, $sx_argv_store");
        }

        for (name, qbe_name) in self.fn_ptr_consts.clone() {
            let tmp = self.fresh_tmp();
            self.emit(&format!("    {tmp} =l copy ${qbe_name}"));
            self.locals.insert(name.clone(), tmp);
            self.local_types.insert(name.clone(), "l");
            self.local_mutability.insert(name.clone(), false);
        }

        self.emit_stmts(&f.body, ns, &f.ret)?;

        if matches!(f.ret, TypeExpr::Void) {
            self.emit_defers(ns, &f.ret)?;
            if qbe_name == "main" {
                self.emit("    ret 0");
            } else {
                self.emit("    ret");
            }
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
        self.type_aliases = self.module_aliases.clone();
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

        self.emit_stmts(body, ns, &TypeExpr::Void)?;

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

    fn emit_stmts(
        &mut self,
        stmts: &[Stmt],
        ns: &Namespace,
        ret_ty: &TypeExpr,
    ) -> Result<(), String> {
        for stmt in stmts {
            self.emit_stmt(stmt, ns, ret_ty)?;
            if let Stmt::When { platform, body } = stmt {
                if self.platform.matches_name(platform) && block_is_terminated(body) {
                    return Ok(());
                }
            }
        }
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
                if let Some(ty) = ty {
                    let inner_ty = if let TypeExpr::Mut(inner) = ty { inner.as_ref() } else { ty };
                    let ann = type_to_annotation_string(inner_ty);
                    if !ann.is_empty() {
                        self.local_type_annotations.insert(name.clone(), ann);
                    }
                } else {
                    let inferred = match expr {
                        Expr::Call { callee, .. } => {
                            let path = expr_to_path(callee);
                            let expanded = expand_alias_path(&path, &self.module_aliases);
                            ns.get(&expanded)
                                .and_then(|qbe_fn| self.fn_ret_type_exprs.get(qbe_fn))
                                .map(type_to_annotation_string)
                                .filter(|s| !s.is_empty())
                        }
                        Expr::Trust(inner) => {
                            if let Expr::Call { callee, .. } = inner.as_ref() {
                                let path = expr_to_path(callee);
                                let expanded = expand_alias_path(&path, &self.module_aliases);
                                ns.get(&expanded)
                                    .and_then(|qbe_fn| self.fn_ret_type_exprs.get(qbe_fn))
                                    .map(type_to_annotation_string)
                                    .filter(|s| !s.is_empty())
                            } else {
                                None
                            }
                        }
                        Expr::Cast { ty, .. } => {
                            let ann = type_to_annotation_string(ty);
                            if ann.is_empty() { None } else { Some(ann) }
                        }
                        _ => None,
                    };
                    if let Some(ann) = inferred {
                        self.local_type_annotations.insert(name.clone(), ann);
                    }
                }
            }

            Stmt::Assign { target, value } => {
                let deref_target = match target {
                    Expr::Deref(inner) => Some(inner.as_ref()),
                    Expr::Trust(inner) => {
                        if let Expr::Deref(d) = inner.as_ref() {
                            Some(d.as_ref())
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                if let Some(ptr_expr) = deref_target {
                    let (ptr, _) = self.emit_expr(ptr_expr, ns)?;
                    let (val_tmp, val_ty) = self.emit_expr(value, ns)?;
                    match val_ty {
                        "d" => self.emit(&format!("    stored {val_tmp}, {ptr}")),
                        "l" => self.emit(&format!("    storel {val_tmp}, {ptr}")),
                        _ => self.emit(&format!("    storew {val_tmp}, {ptr}")),
                    }
                    return Ok(());
                }
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
                            .ok_or_else(|| format!("{}: in fn '{}': undefined variable in assignment target: '{name}'", self.current_file, self.current_fn))?;
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
                    if let Ok(raw) = self.infer_struct_type_name(base) {
                        let type_name = raw.strip_prefix("ref ").unwrap_or(&raw).to_string();
                        let fields = self
                            .type_defs
                            .get(&type_name)
                            .or_else(|| self.extern_type_defs.get(&type_name));
                        if let Some(fields) = fields {
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
            Stmt::GuardedPre(contracts) => {
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
            Stmt::GuardedPost(contracts) => {
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
                self.emit_stmts(then, ns, ret_ty)?;
                if !block_is_terminated(then) {
                    self.emit(&format!("    jmp {end_lbl}"));
                }

                for (i, (elif_cond, elif_body)) in elif_.iter().enumerate() {
                    let (cond_lbl, body_lbl, next_lbl) = &elif_labels[i];
                    self.emit(&format!("{cond_lbl}"));
                    let (ec, _) = self.emit_expr(elif_cond, ns)?;
                    self.emit(&format!("    jnz {ec}, {body_lbl}, {next_lbl}"));
                    self.emit(&format!("{body_lbl}"));
                    self.emit_stmts(elif_body, ns, ret_ty)?;
                    if !block_is_terminated(elif_body) {
                        self.emit(&format!("    jmp {end_lbl}"));
                    }
                }

                if let Some(else_stmts) = else_ {
                    self.emit(&format!("@if_else_{id}"));
                    self.emit_stmts(else_stmts, ns, ret_ty)?;
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
                let cont_lbl = format!("@for_cont_{}", self.tmp_counter);
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
                let prev_loop_cont = self.current_loop_continue.replace(cont_lbl.clone());
                self.emit_stmts(body, ns, ret_ty)?;
                self.current_loop_end = prev_loop_end;
                self.current_loop_continue = prev_loop_cont;

                if !block_is_terminated(body) {
                    self.emit(&format!("    jmp {cont_lbl}"));
                }
                self.emit(&format!("{cont_lbl}"));
                if let Some(post_stmt) = post {
                    self.emit_stmt(post_stmt, ns, ret_ty)?;
                }
                self.emit(&format!("    jmp {loop_lbl}"));
                self.emit(&format!("{end_lbl}"));
            }
            Stmt::ForIn {
                binding,
                iterable,
                body,
            } => {
                let loop_lbl = format!("@forin_loop_{}", self.tmp_counter);
                let body_lbl = format!("@forin_body_{}", self.tmp_counter);
                let end_lbl = format!("@forin_end_{}", self.tmp_counter);
                let cont_lbl = format!("@forin_cont_{}", self.tmp_counter);
                self.tmp_counter += 1;

                let (list_ptr, _) = self.emit_expr(iterable, ns)?;
                let idx_slot = self.fresh_tmp();

                self.emit(&format!("    {idx_slot} =l alloc8 8"));
                self.emit(&format!("    storel 0, {idx_slot}"));

                let len_slot = self.fresh_tmp();

                self.emit(&format!("    {len_slot} =l add {list_ptr}, 8"));
                self.emit(&format!("    jmp {loop_lbl}"));
                self.emit(&format!("{loop_lbl}"));

                let idx_val = self.fresh_tmp();
                let len_val = self.fresh_tmp();

                self.emit(&format!("    {idx_val} =l loadl {idx_slot}"));
                self.emit(&format!("    {len_val} =l loadl {len_slot}"));

                let cond = self.fresh_tmp();

                self.emit(&format!("    {cond} =w csltl {idx_val}, {len_val}"));
                self.emit(&format!("    jnz {cond}, {body_lbl}, {end_lbl}"));
                self.emit(&format!("{body_lbl}"));

                let buf = self.fresh_tmp();
                self.emit(&format!("    {buf} =l loadl {list_ptr}"));
                let elem_off = self.fresh_tmp();
                self.emit(&format!("    {elem_off} =l mul {idx_val}, 8"));
                let elem_ptr = self.fresh_tmp();
                self.emit(&format!("    {elem_ptr} =l add {buf}, {elem_off}"));
                let elem_val = self.fresh_tmp();
                self.emit(&format!("    {elem_val} =l loadl {elem_ptr}"));

                self.locals.insert(binding.clone(), elem_val.clone());
                self.local_types.insert(binding.clone(), "l");
                self.local_mutability.insert(binding.clone(), false);

                let prev_loop_end = self.current_loop_end.replace(end_lbl.clone());
                let prev_loop_cont = self.current_loop_continue.replace(cont_lbl.clone());

                self.emit_stmts(body, ns, ret_ty)?;
                self.current_loop_end = prev_loop_end;
                self.current_loop_continue = prev_loop_cont;

                if !block_is_terminated(body) {
                    self.emit(&format!("    jmp {cont_lbl}"));
                }
                self.emit(&format!("{cont_lbl}"));

                let new_idx = self.fresh_tmp();
                self.emit(&format!("    {new_idx} =l add {idx_val}, 1"));
                self.emit(&format!("    storel {new_idx}, {idx_slot}"));
                self.emit(&format!("    jmp {loop_lbl}"));
                self.emit(&format!("{end_lbl}"));
            }
            Stmt::Increment(var) => {
                let slot = self
                    .locals
                    .get(var)
                    .cloned()
                    .ok_or_else(|| format!("{}: in fn '{}': undefined variable in '{}++': '{var}'", self.current_file, self.current_fn, var))?;
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
            Stmt::Decrement(var) => {
                let slot = self
                    .locals
                    .get(var)
                    .cloned()
                    .ok_or_else(|| format!("{}: in fn '{}': undefined variable in '{}--': '{var}'", self.current_file, self.current_fn, var))?;
                let is_l_slot = self.local_slot_is_l.contains(var);
                let cur = self.fresh_tmp();
                let dec = self.fresh_tmp();
                if is_l_slot {
                    self.emit(&format!("    {cur} =l loadl {slot}"));
                    self.emit(&format!("    {dec} =l sub {cur}, 1"));
                    self.emit(&format!("    storel {dec}, {slot}"));
                } else {
                    self.emit(&format!("    {cur} =w loadw {slot}"));
                    self.emit(&format!("    {dec} =w sub {cur}, 1"));
                    self.emit(&format!("    storew {dec}, {slot}"));
                }
            }
            Stmt::AddAssign(var, expr) => {
                let slot = self
                    .locals
                    .get(var)
                    .cloned()
                    .ok_or_else(|| format!("{}: in fn '{}': undefined variable in '{var} +=': '{var}'", self.current_file, self.current_fn))?;
                let is_l_slot = self.local_slot_is_l.contains(var);
                let (rhs, rhs_ty) = self.emit_expr(expr, ns)?;
                let cur = self.fresh_tmp();
                let res = self.fresh_tmp();
                if is_l_slot {
                    self.emit(&format!("    {cur} =l loadl {slot}"));
                    let (rhs_l, _) = self.promote_to_l(rhs, rhs_ty);
                    self.emit(&format!("    {res} =l add {cur}, {rhs_l}"));
                    self.emit(&format!("    storel {res}, {slot}"));
                } else {
                    self.emit(&format!("    {cur} =w loadw {slot}"));
                    self.emit(&format!("    {res} =w add {cur}, {rhs}"));
                    self.emit(&format!("    storew {res}, {slot}"));
                }
            }
            Stmt::SubAssign(var, expr) => {
                let slot = self
                    .locals
                    .get(var)
                    .cloned()
                    .ok_or_else(|| format!("{}: in fn '{}': undefined variable in '{var} -=': '{var}'", self.current_file, self.current_fn))?;
                let is_l_slot = self.local_slot_is_l.contains(var);
                let (rhs, rhs_ty) = self.emit_expr(expr, ns)?;
                let cur = self.fresh_tmp();
                let res = self.fresh_tmp();
                if is_l_slot {
                    self.emit(&format!("    {cur} =l loadl {slot}"));
                    let (rhs_l, _) = self.promote_to_l(rhs, rhs_ty);
                    self.emit(&format!("    {res} =l sub {cur}, {rhs_l}"));
                    self.emit(&format!("    storel {res}, {slot}"));
                } else {
                    self.emit(&format!("    {cur} =w loadw {slot}"));
                    self.emit(&format!("    {res} =w sub {cur}, {rhs}"));
                    self.emit(&format!("    storew {res}, {slot}"));
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
            Stmt::Continue => {
                let cont_lbl = self
                    .current_loop_continue
                    .clone()
                    .ok_or_else(|| "continue used outside of loop".to_string())?;
                self.emit(&format!("    jmp {cont_lbl}"));
            }
            Stmt::When { platform, body } => {
                let matches = self.platform.matches_name(platform);
                if matches {
                    self.emit_stmts(body, ns, ret_ty)?;
                }
            }
            Stmt::MatchUnion {
                expr,
                arms,
                else_body,
            } => {
                let end_lbl = format!("@union_end_{}", self.tmp_counter);
                self.tmp_counter += 1;

                let (union_ptr, _) = self.emit_expr(expr, ns)?;
                let tag_tmp = self.fresh_tmp();
                self.emit(&format!("    {tag_tmp} =w loadw {union_ptr}"));

                let union_locals: Vec<(String, String)> = self
                    .locals
                    .iter()
                    .filter(|(name, _)| {
                        self.local_type_annotations
                            .get(*name)
                            .map_or(false, |t| self.union_defs.contains_key(t.as_str()))
                    })
                    .map(|(n, v)| (n.clone(), v.clone()))
                    .collect();

                for (i, (ty, body)) in arms.iter().enumerate() {
                    let tag_index = self.resolve_union_tag(expr, ty)?;
                    let body_lbl = format!("@union_arm_{i}_{}", self.tmp_counter - 1);
                    let skip_lbl = format!("@union_skip_{i}_{}", self.tmp_counter - 1);

                    let cond_tmp = self.fresh_tmp();
                    self.emit(&format!("    {cond_tmp} =w ceqw {tag_tmp}, {tag_index}"));
                    self.emit(&format!("    jnz {cond_tmp}, {body_lbl}, {skip_lbl}"));
                    self.emit(&format!("{body_lbl}"));

                    let mut saved: Vec<(String, String)> = Vec::new();
                    for (name, orig) in &union_locals {
                        let payload = self.fresh_tmp();
                        self.emit(&format!("    {payload} =l add {orig}, 8"));
                        saved.push((name.clone(), orig.clone()));
                        self.locals.insert(name.clone(), payload);
                    }

                    self.emit_stmts(body, ns, ret_ty)?;

                    for (name, orig) in saved {
                        self.locals.insert(name, orig);
                    }

                    if !block_is_terminated(body) {
                        self.emit(&format!("    jmp {end_lbl}"));
                    }
                    self.emit(&format!("{skip_lbl}"));
                }

                if let Some(body) = else_body {
                    self.emit_stmts(body, ns, ret_ty)?;
                    if !block_is_terminated(body) {
                        self.emit(&format!("    jmp {end_lbl}"));
                    }
                }

                self.emit(&format!("{end_lbl}"));
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
                let unwrapped = self.fresh_tmp();
                self.emit(&format!("    {unwrapped} =l sub {val_tmp}, 1"));
                self.locals.insert(some_binding.clone(), unwrapped);
                let inner_type_name: Option<String> = match expr {
                    Expr::Call { callee, .. } => {
                        let path = expr_to_path(callee);
                        let expanded = expand_alias_path(&path, &self.module_aliases);
                        ns.get(&expanded)
                            .and_then(|qbe_fn| self.fn_ret_type_exprs.get(qbe_fn))
                            .and_then(|ret| {
                                if let TypeExpr::Option(inner) = ret {
                                    Some(type_to_annotation_string(inner))
                                } else {
                                    None
                                }
                            })
                    }
                    Expr::Trust(inner_expr) => {
                        if let Expr::Call { callee, .. } = inner_expr.as_ref() {
                            let path = expr_to_path(callee);
                            let expanded = expand_alias_path(&path, &self.module_aliases);
                            ns.get(&expanded)
                                .and_then(|qbe_fn| self.fn_ret_type_exprs.get(qbe_fn))
                                .and_then(|ret| {
                                    if let TypeExpr::Option(inner) = ret {
                                        Some(type_to_annotation_string(inner))
                                    } else {
                                        None
                                    }
                                })
                        } else {
                            None
                        }
                    }
                    Expr::Ident(name) => {
                        self.local_type_annotations.get(name).and_then(|ann| {
                            if ann.starts_with("option[") && ann.ends_with(']') {
                                Some(ann[7..ann.len() - 1].to_string())
                            } else {
                                None
                            }
                        })
                    }
                    _ => None,
                };
                if let Some(type_name) = inner_type_name {
                    if !type_name.is_empty() {
                        self.local_type_annotations
                            .insert(some_binding.clone(), type_name);
                    }
                }
                self.emit_stmts(some_body, ns, ret_ty)?;
                let some_terminated = block_is_terminated(some_body);
                if !some_terminated {
                    self.emit(&format!("    jmp {end_lbl}"));
                }

                self.emit(&format!("{none_lbl}"));
                self.emit_stmts(none_body, ns, ret_ty)?;

                let none_terminated = block_is_terminated(none_body);
                if !none_terminated {
                    self.emit(&format!("    jmp {end_lbl}"));
                }
                if !some_terminated || !none_terminated {
                    self.emit(&format!("{end_lbl}"));
                }
            }

            Stmt::MatchResult {
                expr,
                ok_binding,
                ok_body,
                err_binding,
                err_body,
            } => {
                // this is a best effort type scenario
                let ok_is_float = if let Expr::Ident(name) = expr {
                    self.local_type_annotations
                        .get(name)
                        .map(|ann| ann.starts_with("result[f64") || ann == "f64")
                        .unwrap_or(false)
                } else {
                    false
                };

                let (res_ptr, _) = self.emit_expr(expr, ns)?;
                let tag_tmp = self.fresh_tmp();
                self.emit(&format!("    {tag_tmp} =l loadl {res_ptr}"));
                let ok_lbl = format!("@match_ok_{}", self.tmp_counter);
                let err_lbl = format!("@match_err_{}", self.tmp_counter);
                let end_lbl = format!("@match_result_end_{}", self.tmp_counter);
                self.tmp_counter += 1;
                let is_ok = self.fresh_tmp();
                self.emit(&format!("    {is_ok} =w ceql {tag_tmp}, 0"));
                self.emit(&format!("    jnz {is_ok}, {ok_lbl}, {err_lbl}"));

                self.emit(&format!("{ok_lbl}"));
                let val_ptr = self.fresh_tmp();
                self.emit(&format!("    {val_ptr} =l add {res_ptr}, 8"));
                let ok_val = self.fresh_tmp();
                if ok_is_float {
                    self.emit(&format!("    {ok_val} =d loadd {val_ptr}"));
                    self.locals.insert(ok_binding.clone(), ok_val.clone());
                    self.local_types.insert(ok_binding.clone(), "d");
                } else {
                    self.emit(&format!("    {ok_val} =l loadl {val_ptr}"));
                    self.locals.insert(ok_binding.clone(), ok_val.clone());
                    self.local_types.insert(ok_binding.clone(), "l");
                }
                self.emit_stmts(ok_body, ns, ret_ty)?;
                let ok_terminated = block_is_terminated(ok_body);
                if !ok_terminated {
                    self.emit(&format!("    jmp {end_lbl}"));
                }

                self.emit(&format!("{err_lbl}"));
                let err_val_ptr = self.fresh_tmp();
                self.emit(&format!("    {err_val_ptr} =l add {res_ptr}, 8"));
                let err_val = self.fresh_tmp();
                self.emit(&format!("    {err_val} =l loadl {err_val_ptr}"));
                self.locals.insert(err_binding.clone(), err_val);
                self.emit_stmts(err_body, ns, ret_ty)?;
                let err_terminated = block_is_terminated(err_body);
                if !err_terminated {
                    self.emit(&format!("    jmp {end_lbl}"));
                }

                if !ok_terminated || !err_terminated {
                    self.emit(&format!("{end_lbl}"));
                }
            }

            Stmt::MatchEnum { expr, arms } => {
                let (val_tmp, _) = self.emit_expr(expr, ns)?;
                let id = self.tmp_counter;
                self.tmp_counter += 1;
                let end_lbl = format!("@match_enum_end_{id}");

                let n = arms.len();
                for (i, (variant_name, body)) in arms.iter().enumerate() {
                    let variant_idx = self.find_enum_variant_index(variant_name);
                    let body_lbl = format!("@match_enum_arm_{id}_{i}");
                    let next_lbl = if i + 1 < n {
                        format!("@match_enum_next_{id}_{i}")
                    } else {
                        end_lbl.clone()
                    };
                    let cond = self.fresh_tmp();
                    self.emit(&format!("    {cond} =w ceql {val_tmp}, {variant_idx}"));
                    self.emit(&format!("    jnz {cond}, {body_lbl}, {next_lbl}"));
                    self.emit(&format!("{body_lbl}"));
                    self.emit_stmts(body, ns, ret_ty)?;
                    if !block_is_terminated(body) {
                        self.emit(&format!("    jmp {end_lbl}"));
                    }
                    if i + 1 < n {
                        self.emit(&format!("@match_enum_next_{id}_{i}"));
                    }
                }
                self.emit(&format!("{end_lbl}"));
            }
            Stmt::MatchString {
                expr,
                arms,
                else_body,
            } => {
                let (str_ptr, _) = self.emit_expr(expr, ns)?;
                let id = self.tmp_counter;
                self.tmp_counter += 1;
                let end_lbl = format!("@match_string_end_{id}");

                let n = arms.len();
                for (i, (pattern, body)) in arms.iter().enumerate() {
                    let body_lbl = format!("@match_string_arm_{id}_{i}");
                    let next_lbl = if i + 1 < n {
                        format!("@match_string_next_{id}_{i}")
                    } else {
                        if else_body.is_some() {
                            format!("@match_string_else_{id}")
                        } else {
                            end_lbl.clone()
                        }
                    };

                    let pattern_label = self.intern_string(pattern);
                    let cmp_result = self.fresh_tmp();
                    self.emit(&format!(
                        "    {cmp_result} =w call $strcmp(l {str_ptr}, l ${pattern_label})"
                    ));

                    let cond = self.fresh_tmp();
                    self.emit(&format!("    {cond} =w ceqw {cmp_result}, 0"));
                    self.emit(&format!("    jnz {cond}, {body_lbl}, {next_lbl}"));
                    self.emit(&format!("{body_lbl}"));
                    self.emit_stmts(body, ns, ret_ty)?;
                    if !block_is_terminated(body) {
                        self.emit(&format!("    jmp {end_lbl}"));
                    }
                    if i + 1 < n {
                        self.emit(&format!("@match_string_next_{id}_{i}"));
                    }
                }

                if let Some(body) = else_body {
                    if n > 0 {
                        self.emit(&format!("@match_string_else_{id}"));
                    }
                    self.emit_stmts(body, ns, ret_ty)?;
                    if !block_is_terminated(body) {
                        self.emit(&format!("    jmp {end_lbl}"));
                    }
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
            self.emit_stmts(body, ns, ret_ty)?;
        }
        Ok(())
    }

    /// Find the index of an enum variant by name, searching all known enum definitions.
    fn find_enum_variant_index(&self, variant_name: &str) -> usize {
        for variants in self.enum_defs.values() {
            if let Some(idx) = variants.iter().position(|v| v == variant_name) {
                return idx;
            }
        }
        0
    }

    /// Compute a pointer to a field within a struct.
    /// `expr` must be of the form `base.field` or `base.field.field...`
    fn emit_field_ptr(&mut self, expr: &Expr, ns: &Namespace) -> Result<String, String> {
        match expr {
            Expr::Field(base, field_name) => {
                let base_ptr = match base.as_ref() {
                    Expr::Ident(name) => {
                        let slot = self
                            .locals
                            .get(name)
                            .cloned()
                            .ok_or_else(|| format!("{}: in fn '{}': undefined variable in field access '.{}': '{name}'", self.current_file, self.current_fn, field_name))?;
                        if self.local_is_slot.contains(name) {
                            let is_struct_slot = self
                                .local_type_annotations
                                .get(name)
                                .map(|ann| {
                                    let type_name = ann.strip_prefix("ref ").unwrap_or(ann);
                                    self.type_defs.contains_key(type_name)
                                        || self.extern_type_defs.contains_key(type_name)
                                })
                                .unwrap_or(false);
                            if is_struct_slot {
                                let ptr = self.fresh_tmp();
                                self.emit(&format!("    {ptr} =l loadl {slot}"));
                                ptr
                            } else {
                                slot
                            }
                        } else {
                            slot
                        }
                    }
                    Expr::Cast { expr: inner, .. } => {
                        let (tmp, _) = self.emit_expr(inner, ns)?;
                        tmp
                    }
                    other => self.emit_field_ptr(other, ns)?,
                };

                let offset = self.field_offset_for(base, field_name)?;
                let ptr = self.fresh_tmp();
                self.emit(&format!("    {ptr} =l add {base_ptr}, {offset}"));
                Ok(ptr)
            }
            _ => Err(format!(
                "in function '{}': expected field access (e.g. `x.field`) as assignment target, \
                 but got: {:?}",
                self.current_fn, expr
            )),
        }
    }

    /// Return the byte offset of `field_name` within the struct that `base` refers to.
    /// We look up the binding's declared type annotation to find the type definition.
    fn field_offset_for(&self, base: &Expr, field_name: &str) -> Result<usize, String> {
        let raw_type_name = self.infer_struct_type_name(base)?;
        let type_name = raw_type_name
            .strip_prefix("ref ")
            .unwrap_or(&raw_type_name)
            .to_string();
        let bare_name = type_name.rsplit('.').next().unwrap_or(&type_name).to_string();
        let fields = self
            .type_defs
            .get(&type_name)
            .or_else(|| self.extern_type_defs.get(&type_name))
            .or_else(|| self.type_defs.get(&bare_name))
            .or_else(|| self.extern_type_defs.get(&bare_name))
            .ok_or_else(|| format!("unknown type '{type_name}'"))?;

        let has_fixed_arrays = fields
            .iter()
            .any(|f| matches!(f.ty, TypeExpr::FixedArray(_, _)));

        let mut offset: usize = 0;
        for field in fields {
            if field.name == field_name {
                return Ok(offset);
            }
            let field_size = type_byte_size(&field.ty);
            if has_fixed_arrays {
                let align = type_alignment(&field.ty);
                offset = align_to(offset as u64, align) as usize + field_size as usize;
            } else {
                offset += align8(field_size) as usize;
            }
        }
        Err(format!("type '{type_name}' has no field '{field_name}'"))
    }

    /// Try to infer the struct type name of an expression (best-effort, ident only).
    fn infer_struct_type_name(&self, expr: &Expr) -> Result<String, String> {
        match expr {
            Expr::Ident(name) => self
                .local_type_annotations
                .get(name)
                .cloned()
                .ok_or_else(|| format!("cannot determine type of '{name}'")),
            Expr::Cast { ty, .. } => Ok(type_to_annotation_string(ty)),
            _ => Err("cannot determine struct type for complex expression".into()),
        }
    }

    /// Try to infer the type of a field within a struct.
    fn infer_field_type(&self, base: &Expr, field_name: &str) -> Option<TypeExpr> {
        let raw = self.infer_struct_type_name(base).ok()?;
        let type_name = raw.strip_prefix("ref ").unwrap_or(&raw).to_string();
        let bare_name = type_name.rsplit('.').next().unwrap_or(&type_name).to_string();
        let fields = self
            .type_defs
            .get(&type_name)
            .or_else(|| self.extern_type_defs.get(&type_name))
            .or_else(|| self.type_defs.get(&bare_name))
            .or_else(|| self.extern_type_defs.get(&bare_name))?;
        fields
            .iter()
            .find(|f| f.name == field_name)
            .map(|f| f.ty.clone())
    }

    /// Try to infer the struct type name from a struct literal by matching field names.
    fn infer_struct_type_from_lit(&self, fields: &[(String, Expr)]) -> Option<String> {
        if fields.is_empty() {
            return None;
        }
        let first_field_name = &fields[0].0;

        for type_name in self.type_defs.keys().chain(self.extern_type_defs.keys()) {
            let field_defs = self
                .type_defs
                .get(type_name)
                .or_else(|| self.extern_type_defs.get(type_name));
            if let Some(field_defs) = field_defs {
                if field_defs.iter().any(|f| f.name == *first_field_name) {
                    let matches = fields
                        .iter()
                        .all(|(fname, _)| field_defs.iter().any(|f| f.name == *fname));
                    if matches {
                        return Some(type_name.clone());
                    }
                }
            }
        }
        None
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

    /// Best-effort check: is this expression float-typed?
    /// Used to decide whether to use loadd/ceqd vs loadl/ceql for Result payload comparison.
    fn expr_is_float(&self, expr: &Expr) -> bool {
        match expr {
            Expr::FloatLit(_) => true,
            Expr::Cast { ty, .. } => matches!(ty, TypeExpr::Named(n) if n == "f64"),
            Expr::Ident(name) => {
                self.local_slot_is_d.contains(name)
                    || self.local_types.get(name).copied() == Some("d")
                    || self
                        .local_type_annotations
                        .get(name)
                        .map_or(false, |t| t == "f64")
            }
            Expr::OkVal(inner) | Expr::ErrVal(inner) => self.expr_is_float(inner),
            _ => false,
        }
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
                if let Some(slot_or_tmp) = self.locals.get(name).cloned() {
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
                } else if let Some(qbe_name) = ns.get(name) {
                    let tmp = self.fresh_tmp();
                    self.emit(&format!("    {tmp} =l copy ${qbe_name}"));
                    Ok((tmp, "l"))
                } else {
                    Err(format!("{}: undefined variable: {name}", self.current_file))
                }
            }

            Expr::Bool(b) => Ok((if *b { "1".into() } else { "0".into() }, "w")),
            Expr::None => Ok(("0".into(), "l")),
            Expr::Some(inner) => {
                let (tmp, ty) = self.emit_expr(inner, ns)?;
                let (tmp_l, _) = self.promote_to_l(tmp, ty);
                let result = self.fresh_tmp();
                self.emit(&format!("    {result} =l add {tmp_l}, 1"));
                Ok((result, "l"))
            }
            Expr::OkVal(inner) => {
                let (val, val_ty) = self.emit_expr(inner, ns)?;
                let ptr = self.fresh_tmp();
                self.emit(&format!("    {ptr} =l call $malloc(l 16)"));
                self.emit(&format!("    storel 0, {ptr}"));
                let val_ptr = self.fresh_tmp();
                self.emit(&format!("    {val_ptr} =l add {ptr}, 8"));
                if val_ty == "d" {
                    self.emit(&format!("    stored {val}, {val_ptr}"));
                } else {
                    let (val_l, _) = self.promote_to_l(val, val_ty);
                    self.emit(&format!("    storel {val_l}, {val_ptr}"));
                }
                Ok((ptr, "l"))
            }
            Expr::ErrVal(inner) => {
                let (val, val_ty) = self.emit_expr(inner, ns)?;
                let ptr = self.fresh_tmp();
                self.emit(&format!("    {ptr} =l call $malloc(l 16)"));
                self.emit(&format!("    storel 1, {ptr}"));
                let val_ptr = self.fresh_tmp();
                self.emit(&format!("    {val_ptr} =l add {ptr}, 8"));
                if val_ty == "d" {
                    self.emit(&format!("    stored {val}, {val_ptr}"));
                } else {
                    let (val_l, _) = self.promote_to_l(val, val_ty);
                    self.emit(&format!("    storel {val_l}, {val_ptr}"));
                }
                Ok((ptr, "l"))
            }
            Expr::Try(inner) => {
                let (res_ptr, _) = self.emit_expr(inner, ns)?;
                let tag_tmp = self.fresh_tmp();
                self.emit(&format!("    {tag_tmp} =l loadl {res_ptr}"));
                let ok_lbl = format!("@try_ok_{}", self.tmp_counter);
                let err_lbl = format!("@try_err_{}", self.tmp_counter);
                self.tmp_counter += 1;
                let is_ok = self.fresh_tmp();
                self.emit(&format!("    {is_ok} =w ceql {tag_tmp}, 0"));
                self.emit(&format!("    jnz {is_ok}, {ok_lbl}, {err_lbl}"));
                self.emit(&format!("{err_lbl}"));
                let fn_ret = self.current_fn_ret.clone();
                self.emit_defers(ns, &fn_ret)?;
                self.emit(&format!("    ret {res_ptr}"));
                self.emit(&format!("{ok_lbl}"));

                // Check if the inner call returns result[void, E] — if so, no value to extract.
                let is_void_ok = match inner.as_ref() {
                    Expr::Call { callee, .. } => {
                        if let Ok(name) = resolve_call_name(callee, ns, &self.type_aliases) {
                            self.result_void_ok.contains(&name)
                        } else {
                            false
                        }
                    }
                    _ => false,
                };

                if is_void_ok {
                    // result[void, E]: no ok value, return a dummy
                    Ok(("0".into(), "w"))
                } else {
                    let val_ptr = self.fresh_tmp();
                    self.emit(&format!("    {val_ptr} =l add {res_ptr}, 8"));
                    let val = self.fresh_tmp();
                    self.emit(&format!("    {val} =l loadl {val_ptr}"));
                    Ok((val, "l"))
                }
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
                "sizeof" => {
                    let type_name = match &args[0] {
                        Expr::Ident(n) => n.clone(),
                        other => return Err(format!("@sizeof expects a type name, got {other:?}")),
                    };
                    let size = if let Some(fields) = self.type_defs.get(&type_name)
                        .or_else(|| self.extern_type_defs.get(&type_name))
                    {
                        let has_fixed_arrays = fields.iter().any(|f| matches!(f.ty, TypeExpr::FixedArray(_, _)));
                        let mut total: u64 = 0;
                        for field in fields {
                            let field_size = type_byte_size(&field.ty);
                            if has_fixed_arrays {
                                let align = type_alignment(&field.ty);
                                total = align_to(total, align) + field_size;
                            } else {
                                total += align8(field_size);
                            }
                        }
                        total
                    } else {
                        match type_name.as_str() {
                            "i8" | "u8" | "char" => 1,
                            "i16" | "u16" => 2,
                            "i32" | "u32" | "f32" | "bool" => 4,
                            _ => 8,
                        }
                    };
                    Ok((size.to_string(), "l"))
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
                "memcpy" => {
                    let (dst, _) = self.emit_expr(&args[0], ns)?;
                    let (src, _) = self.emit_expr(&args[1], ns)?;
                    let (size, size_ty) = self.emit_expr(&args[2], ns)?;
                    let (size_l, _) = self.promote_to_l(size, size_ty);
                    let tmp = self.fresh_tmp();
                    self.emit(&format!(
                        "    {tmp} =l call $memcpy(l {dst}, l {src}, l {size_l})"
                    ));
                    Ok((tmp, "l"))
                }
                "load8" => {
                    let (ptr, _) = self.emit_expr(&args[0], ns)?;
                    let tmp = self.fresh_tmp();
                    self.emit(&format!("    {tmp} =w loadub {ptr}"));
                    Ok((tmp, "w"))
                }
                "store8" => {
                    let (ptr, _) = self.emit_expr(&args[0], ns)?;
                    let (val, val_ty) = self.emit_expr(&args[1], ns)?;
                    let val_w = if val_ty == "l" {
                        let w = self.fresh_tmp();
                        self.emit(&format!("    {w} =w copy {val}"));
                        w
                    } else {
                        val
                    };
                    self.emit(&format!("    storeb {val_w}, {ptr}"));
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
                "fmt" => {
                    if args.is_empty() {
                        return Err("@fmt requires a format string argument".into());
                    }
                    let fmt_tmp = match &args[0] {
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
                    let buf = self.fresh_tmp();
                    self.emit(&format!("    {buf} =l call $malloc(l 4096)"));
                    let written_w = self.fresh_tmp();
                    if variadic_args.is_empty() {
                        self.emit(&format!(
                            "    {written_w} =w call $snprintf(l {buf}, l 4096, l {fmt_tmp})"
                        ));
                    } else {
                        self.emit(&format!(
                            "    {written_w} =w call $snprintf(l {buf}, l 4096, l {fmt_tmp}, ..., {})",
                            variadic_args.join(", ")
                        ));
                    }
                    let written = self.fresh_tmp();
                    self.emit(&format!("    {written} =l extsw {written_w}"));
                    let copy_size = self.fresh_tmp();
                    self.emit(&format!("    {copy_size} =l add {written}, 1"));
                    let copy = self.fresh_tmp();
                    self.emit(&format!("    {copy} =l call $malloc(l {copy_size})"));
                    self.emit(&format!("    call $memcpy(l {copy}, l {buf}, l {written})"));
                    let nul_ptr = self.fresh_tmp();
                    self.emit(&format!("    {nul_ptr} =l add {copy}, {written}"));
                    self.emit(&format!("    storeb 0, {nul_ptr}"));
                    self.emit(&format!("    call $free(l {buf})"));
                    Ok((copy, "l"))
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
                "call" => {
                    if args.is_empty() {
                        return Err("@call requires at least a function pointer argument".into());
                    }
                    let (fn_ptr, _) = self.emit_expr(&args[0], ns)?;
                    let mut call_args = Vec::new();
                    for a in args.iter().skip(1) {
                        let (tmp, ty) = self.emit_expr(a, ns)?;
                        let (promoted, pty) = self.promote_to_l(tmp, ty);
                        call_args.push(format!("{pty} {promoted}"));
                    }
                    let tmp = self.fresh_tmp();
                    if call_args.is_empty() {
                        self.emit(&format!("    {tmp} =l call {fn_ptr}()"));
                    } else {
                        self.emit(&format!(
                            "    {tmp} =l call {fn_ptr}({})",
                            call_args.join(", ")
                        ));
                    }
                    Ok((tmp, "l"))
                }
                "args" => {
                    let tmp = self.fresh_tmp();
                    self.emit(&format!("    {tmp} =l call $sx_get_args()"));
                    self.local_type_annotations
                        .insert(tmp.clone(), "list[ref char]".to_string());
                    Ok((tmp, "l"))
                }

                "append" => {
                    if args.len() != 2 {
                        return Err("@append(list, value) requires exactly 2 arguments".into());
                    }
                    let (list_ptr, _) = self.emit_expr(&args[0], ns)?;
                    let (val, val_ty) = self.emit_expr(&args[1], ns)?;

                    let len_slot = self.fresh_tmp();
                    let cap_slot = self.fresh_tmp();
                    self.emit(&format!("    {len_slot} =l add {list_ptr}, 8"));
                    self.emit(&format!("    {cap_slot} =l add {list_ptr}, 16"));

                    let len_val = self.fresh_tmp();
                    let cap_val = self.fresh_tmp();
                    self.emit(&format!("    {len_val} =l loadl {len_slot}"));
                    self.emit(&format!("    {cap_val} =l loadl {cap_slot}"));

                    let need_grow = self.fresh_tmp();
                    let grow_lbl = format!("@append_grow_{}", self.tmp_counter);
                    let no_grow_lbl = format!("@append_nogrow_{}", self.tmp_counter);
                    self.tmp_counter += 1;
                    self.emit(&format!("    {need_grow} =w csgel {len_val}, {cap_val}"));
                    self.emit(&format!("    jnz {need_grow}, {grow_lbl}, {no_grow_lbl}"));
                    self.emit(&format!("{grow_lbl}"));
                    let new_cap = self.fresh_tmp();
                    self.emit(&format!("    {new_cap} =l mul {cap_val}, 2"));
                    let new_cap = format!("{new_cap}");
                    self.emit(&format!("    {new_cap} =l add {new_cap}, 1"));
                    let old_buf = self.fresh_tmp();
                    let new_buf = self.fresh_tmp();
                    self.emit(&format!("    {old_buf} =l loadl {list_ptr}"));
                    let new_cap_bytes = self.fresh_tmp();
                    self.emit(&format!("    {new_cap_bytes} =l mul {new_cap}, 8"));
                    self.emit(&format!(
                        "    {new_buf} =l call $realloc(l {old_buf}, l {new_cap_bytes})"
                    ));
                    self.emit(&format!("    storel {new_buf}, {list_ptr}"));
                    self.emit(&format!("    storel {new_cap}, {cap_slot}"));
                    self.emit(&format!("    jmp {no_grow_lbl}"));
                    self.emit(&format!("{no_grow_lbl}"));

                    let buf = self.fresh_tmp();
                    let off = self.fresh_tmp();
                    let elem_ptr = self.fresh_tmp();
                    self.emit(&format!("    {buf} =l loadl {list_ptr}"));
                    self.emit(&format!("    {off} =l mul {len_val}, 8"));
                    self.emit(&format!("    {elem_ptr} =l add {buf}, {off}"));
                    let (store_val, _) = self.promote_to_l(val, val_ty);
                    self.emit(&format!("    storel {store_val}, {elem_ptr}"));

                    let new_len = self.fresh_tmp();
                    self.emit(&format!("    {new_len} =l add {len_val}, 1"));
                    self.emit(&format!("    storel {new_len}, {len_slot}"));

                    Ok(("0".into(), "w"))
                }

                "get" => {
                    if args.len() != 2 {
                        return Err("@get(list, index) requires exactly 2 arguments".into());
                    }
                    let (list_ptr, _) = self.emit_expr(&args[0], ns)?;
                    let (idx, idx_ty) = self.emit_expr(&args[1], ns)?;
                    let (idx_l, _) = self.promote_to_l(idx, idx_ty);

                    let len_slot = self.fresh_tmp();
                    let len_val = self.fresh_tmp();
                    self.emit(&format!("    {len_slot} =l add {list_ptr}, 8"));
                    self.emit(&format!("    {len_val} =l loadl {len_slot}"));

                    let in_bounds = self.fresh_tmp();
                    let some_lbl = format!("@get_some_{}", self.tmp_counter);
                    let none_lbl = format!("@get_none_{}", self.tmp_counter);
                    let end_lbl = format!("@get_end_{}", self.tmp_counter);
                    self.tmp_counter += 1;
                    self.emit(&format!("    {in_bounds} =w csltl {idx_l}, {len_val}"));
                    self.emit(&format!("    jnz {in_bounds}, {some_lbl}, {none_lbl}"));

                    self.emit(&format!("{some_lbl}"));
                    let buf = self.fresh_tmp();
                    let off = self.fresh_tmp();
                    let elem_ptr = self.fresh_tmp();
                    let elem_val = self.fresh_tmp();
                    let result = self.fresh_tmp();
                    self.emit(&format!("    {buf} =l loadl {list_ptr}"));
                    self.emit(&format!("    {off} =l mul {idx_l}, 8"));
                    self.emit(&format!("    {elem_ptr} =l add {buf}, {off}"));
                    self.emit(&format!("    {elem_val} =l loadl {elem_ptr}"));
                    self.emit(&format!("    {result} =l add {elem_val}, 1"));
                    self.emit(&format!("    jmp {end_lbl}"));

                    self.emit(&format!("{none_lbl}"));
                    let result_none = self.fresh_tmp();
                    self.emit(&format!("    {result_none} =l copy 0"));
                    self.emit(&format!("    jmp {end_lbl}"));

                    self.emit(&format!("{end_lbl}"));

                    Ok((format!("{result}"), "l"))
                }

                "len" => {
                    if args.len() != 1 {
                        return Err("@len(list) requires exactly 1 argument".into());
                    }
                    let (list_ptr, _) = self.emit_expr(&args[0], ns)?;
                    let len_slot = self.fresh_tmp();
                    let len_val = self.fresh_tmp();
                    self.emit(&format!("    {len_slot} =l add {list_ptr}, 8"));
                    self.emit(&format!("    {len_val} =l loadl {len_slot}"));
                    Ok((len_val, "l"))
                }

                "remove" => {
                    if args.len() != 2 {
                        return Err("@remove(list, index) requires exactly 2 arguments".into());
                    }
                    let (list_ptr, _) = self.emit_expr(&args[0], ns)?;
                    let (idx, idx_ty) = self.emit_expr(&args[1], ns)?;
                    let (idx_l, _) = self.promote_to_l(idx, idx_ty);

                    let len_slot = self.fresh_tmp();
                    let cap_slot = self.fresh_tmp();
                    let len_val = self.fresh_tmp();
                    self.emit(&format!("    {len_slot} =l add {list_ptr}, 8"));
                    self.emit(&format!("    {cap_slot} =l add {list_ptr}, 16"));
                    self.emit(&format!("    {len_val} =l loadl {len_slot}"));

                    let buf = self.fresh_tmp();
                    let off = self.fresh_tmp();
                    let elem_ptr = self.fresh_tmp();
                    let elem_val = self.fresh_tmp();
                    self.emit(&format!("    {buf} =l loadl {list_ptr}"));
                    self.emit(&format!("    {off} =l mul {idx_l}, 8"));
                    self.emit(&format!("    {elem_ptr} =l add {buf}, {off}"));
                    self.emit(&format!("    {elem_val} =l loadl {elem_ptr}"));

                    let shift_loop = format!("@remove_shift_{}", self.tmp_counter);
                    let shift_body = format!("@remove_shift_body_{}", self.tmp_counter);
                    let shift_end = format!("@remove_shift_end_{}", self.tmp_counter);
                    self.tmp_counter += 1;

                    let shift_idx = self.fresh_tmp();
                    self.emit(&format!("    {shift_idx} =l copy {idx_l}"));
                    self.emit(&format!("    jmp {shift_loop}"));
                    self.emit(&format!("{shift_loop}"));
                    let one_past_end = self.fresh_tmp();
                    let should_shift = self.fresh_tmp();
                    self.emit(&format!("    {one_past_end} =l sub {len_val}, 1"));
                    self.emit(&format!(
                        "    {should_shift} =w csltl {shift_idx}, {one_past_end}"
                    ));
                    self.emit(&format!(
                        "    jnz {should_shift}, {shift_body}, {shift_end}"
                    ));
                    self.emit(&format!("{shift_body}"));
                    let src_off = self.fresh_tmp();
                    let dst_off = self.fresh_tmp();
                    let src_ptr = self.fresh_tmp();
                    let dst_ptr = self.fresh_tmp();
                    let src_val = self.fresh_tmp();
                    self.emit(&format!("    {src_off} =l add {shift_idx}, 1"));
                    self.emit(&format!("    {src_off} =l mul {src_off}, 8"));
                    self.emit(&format!("    {dst_off} =l mul {shift_idx}, 8"));
                    self.emit(&format!("    {src_ptr} =l add {buf}, {src_off}"));
                    self.emit(&format!("    {dst_ptr} =l add {buf}, {dst_off}"));
                    self.emit(&format!("    {src_val} =l loadl {src_ptr}"));
                    self.emit(&format!("    storel {src_val}, {dst_ptr}"));
                    let shift_idx_new = self.fresh_tmp();
                    self.emit(&format!("    {shift_idx_new} =l add {shift_idx}, 1"));
                    self.emit(&format!("    {shift_idx} =l copy {shift_idx_new}"));
                    self.emit(&format!("    jmp {shift_loop}"));
                    self.emit(&format!("{shift_end}"));

                    let new_len = self.fresh_tmp();
                    self.emit(&format!("    {new_len} =l sub {len_val}, 1"));
                    self.emit(&format!("    storel {new_len}, {len_slot}"));

                    Ok((elem_val, "l"))
                }

                "insert" => {
                    if args.len() != 3 {
                        return Err(
                            "@insert(list, index, value) requires exactly 3 arguments".into()
                        );
                    }
                    let (list_ptr, _) = self.emit_expr(&args[0], ns)?;
                    let (idx, idx_ty) = self.emit_expr(&args[1], ns)?;
                    let (idx_l, _) = self.promote_to_l(idx, idx_ty);
                    let (val, val_ty) = self.emit_expr(&args[2], ns)?;
                    let (val_l, _) = self.promote_to_l(val, val_ty);

                    let len_slot = self.fresh_tmp();
                    let cap_slot = self.fresh_tmp();
                    let len_val = self.fresh_tmp();
                    let cap_val = self.fresh_tmp();
                    self.emit(&format!("    {len_slot} =l add {list_ptr}, 8"));
                    self.emit(&format!("    {cap_slot} =l add {list_ptr}, 16"));
                    self.emit(&format!("    {len_val} =l loadl {len_slot}"));
                    self.emit(&format!("    {cap_val} =l loadl {cap_slot}"));

                    let need_grow = self.fresh_tmp();
                    let grow_lbl = format!("@insert_grow_{}", self.tmp_counter);
                    let no_grow_lbl = format!("@insert_nogrow_{}", self.tmp_counter);
                    self.tmp_counter += 1;
                    self.emit(&format!("    {need_grow} =w csgel {len_val}, {cap_val}"));
                    self.emit(&format!("    jnz {need_grow}, {grow_lbl}, {no_grow_lbl}"));
                    self.emit(&format!("{grow_lbl}"));
                    let new_cap = self.fresh_tmp();
                    self.emit(&format!("    {new_cap} =l mul {cap_val}, 2"));
                    let new_cap2 = self.fresh_tmp();
                    self.emit(&format!("    {new_cap2} =l add {new_cap}, 1"));
                    let old_buf = self.fresh_tmp();
                    let new_buf = self.fresh_tmp();
                    self.emit(&format!("    {old_buf} =l loadl {list_ptr}"));
                    self.emit(&format!(
                        "    {new_buf} =l call $realloc(l {old_buf}, l {new_cap2})"
                    ));
                    self.emit(&format!("    storel {new_buf}, {list_ptr}"));
                    self.emit(&format!("    storel {new_cap2}, {cap_slot}"));
                    self.emit(&format!("    jmp {no_grow_lbl}"));
                    self.emit(&format!("{no_grow_lbl}"));

                    let shift_loop = format!("@insert_shift_{}", self.tmp_counter);
                    let shift_body = format!("@insert_shift_body_{}", self.tmp_counter);
                    let shift_end = format!("@insert_shift_end_{}", self.tmp_counter);
                    self.tmp_counter += 1;

                    let shift_idx = self.fresh_tmp();
                    self.emit(&format!("    {shift_idx} =l sub {len_val}, 1"));
                    self.emit(&format!("    jmp {shift_loop}"));
                    self.emit(&format!("{shift_loop}"));
                    let should_shift = self.fresh_tmp();
                    self.emit(&format!("    {should_shift} =w cslel {idx_l}, {shift_idx}"));
                    self.emit(&format!(
                        "    jnz {should_shift}, {shift_body}, {shift_end}"
                    ));
                    self.emit(&format!("{shift_body}"));
                    let src_off = self.fresh_tmp();
                    let dst_off = self.fresh_tmp();
                    let src_ptr = self.fresh_tmp();
                    let dst_ptr = self.fresh_tmp();
                    let src_val = self.fresh_tmp();
                    self.emit(&format!("    {src_off} =l mul {shift_idx}, 8"));
                    self.emit(&format!("    {dst_off} =l add {shift_idx}, 1"));
                    self.emit(&format!("    {dst_off} =l mul {dst_off}, 8"));
                    let buf2 = self.fresh_tmp();
                    self.emit(&format!("    {buf2} =l loadl {list_ptr}"));
                    self.emit(&format!("    {src_ptr} =l add {buf2}, {src_off}"));
                    self.emit(&format!("    {dst_ptr} =l add {buf2}, {dst_off}"));
                    self.emit(&format!("    {src_val} =l loadl {src_ptr}"));
                    self.emit(&format!("    storel {src_val}, {dst_ptr}"));
                    let shift_idx_new = self.fresh_tmp();
                    self.emit(&format!("    {shift_idx_new} =l sub {shift_idx}, 1"));
                    self.emit(&format!("    {shift_idx} =l copy {shift_idx_new}"));
                    self.emit(&format!("    jmp {shift_loop}"));
                    self.emit(&format!("{shift_end}"));

                    // Store value at idx
                    let buf3 = self.fresh_tmp();
                    let elem_off = self.fresh_tmp();
                    let elem_ptr = self.fresh_tmp();
                    self.emit(&format!("    {buf3} =l loadl {list_ptr}"));
                    self.emit(&format!("    {elem_off} =l mul {idx_l}, 8"));
                    self.emit(&format!("    {elem_ptr} =l add {buf3}, {elem_off}"));
                    self.emit(&format!("    storel {val_l}, {elem_ptr}"));

                    // len = len + 1
                    let new_len = self.fresh_tmp();
                    self.emit(&format!("    {new_len} =l add {len_val}, 1"));
                    self.emit(&format!("    storel {new_len}, {len_slot}"));

                    Ok(("0".into(), "w"))
                }

                "reserve" => {
                    if args.len() != 2 {
                        return Err("@reserve(list, capacity) requires exactly 2 arguments".into());
                    }
                    let (list_ptr, _) = self.emit_expr(&args[0], ns)?;
                    let (new_cap, new_cap_ty) = self.emit_expr(&args[1], ns)?;
                    let (new_cap_l, _) = self.promote_to_l(new_cap, new_cap_ty);

                    let cap_slot = self.fresh_tmp();
                    let cur_cap = self.fresh_tmp();
                    self.emit(&format!("    {cap_slot} =l add {list_ptr}, 16"));
                    self.emit(&format!("    {cur_cap} =l loadl {cap_slot}"));

                    let need_grow = self.fresh_tmp();
                    let grow_lbl = format!("@reserve_grow_{}", self.tmp_counter);
                    let end_lbl = format!("@reserve_end_{}", self.tmp_counter);
                    self.tmp_counter += 1;
                    self.emit(&format!("    {need_grow} =w csgtl {new_cap_l}, {cur_cap}"));
                    self.emit(&format!("    jnz {need_grow}, {grow_lbl}, {end_lbl}"));
                    self.emit(&format!("{grow_lbl}"));
                    let old_buf = self.fresh_tmp();
                    let new_buf = self.fresh_tmp();
                    let new_bytes = self.fresh_tmp();
                    self.emit(&format!("    {old_buf} =l loadl {list_ptr}"));
                    self.emit(&format!("    {new_bytes} =l mul {new_cap_l}, 8"));
                    self.emit(&format!(
                        "    {new_buf} =l call $realloc(l {old_buf}, l {new_bytes})"
                    ));
                    self.emit(&format!("    storel {new_buf}, {list_ptr}"));
                    self.emit(&format!("    storel {new_cap_l}, {cap_slot}"));
                    self.emit(&format!("{end_lbl}"));

                    Ok(("0".into(), "w"))
                }

                "capacity" => {
                    if args.len() != 1 {
                        return Err("@capacity(list) requires exactly 1 argument".into());
                    }
                    let (list_ptr, _) = self.emit_expr(&args[0], ns)?;
                    let cap_slot = self.fresh_tmp();
                    let cap_val = self.fresh_tmp();
                    self.emit(&format!("    {cap_slot} =l add {list_ptr}, 16"));
                    self.emit(&format!("    {cap_val} =l loadl {cap_slot}"));
                    Ok((cap_val, "l"))
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
                let path = expr_to_path(expr);
                let expanded = expand_alias_path(&path, &self.type_aliases);
                if let Some((val, ty)) = self.cross_module_consts.get(&expanded).cloned() {
                    return Ok((val, ty));
                }
                let ptr = self.emit_field_ptr(expr, ns)?;

                let is_fixed_array_field = self
                    .infer_field_type(_base, _field_name)
                    .map(|ty| matches!(ty, TypeExpr::FixedArray(_, _)))
                    .unwrap_or(false);

                if is_fixed_array_field {
                    Ok((ptr, "l"))
                } else {
                    let tmp = self.fresh_tmp();
                    self.emit(&format!("    {tmp} =l loadl {ptr}"));
                    Ok((tmp, "l"))
                }
            }

            Expr::StructLit { fields } => {
                let (total_size, field_defs_opt) = if let Some(type_name) =
                    self.infer_struct_type_from_lit(fields)
                {
                    if let Some(field_defs) = self
                        .type_defs
                        .get(&type_name)
                        .or_else(|| self.extern_type_defs.get(&type_name))
                    {
                        let has_fixed_arrays = field_defs
                            .iter()
                            .any(|f| matches!(f.ty, TypeExpr::FixedArray(_, _)));

                        let mut size: usize = 0;
                        for field in field_defs {
                            let field_size = type_byte_size(&field.ty);
                            if has_fixed_arrays {
                                let align = type_alignment(&field.ty);
                                size = align_to(size as u64, align) as usize + field_size as usize;
                            } else {
                                size += align8(field_size) as usize;
                            }
                        }
                        (size, Some(field_defs.clone()))
                    } else {
                        (fields.len() * 8, None)
                    }
                } else {
                    (fields.len() * 8, None)
                };

                let ptr = self.fresh_tmp();
                self.emit(&format!("    {ptr} =l alloc8 {total_size}"));

                let (field_offsets, field_types): (Vec<usize>, Vec<Option<TypeExpr>>) =
                    if let Some(ref field_defs) = field_defs_opt {
                        let has_fixed_arrays = field_defs
                            .iter()
                            .any(|f| matches!(f.ty, TypeExpr::FixedArray(_, _)));
                        let mut offsets = Vec::new();
                        let mut types = Vec::new();
                        let mut offset: usize = 0;
                        for field in field_defs {
                            offsets.push(offset);
                            types.push(Some(field.ty.clone()));
                            let field_size = type_byte_size(&field.ty);
                            if has_fixed_arrays {
                                let align = type_alignment(&field.ty);
                                offset =
                                    align_to(offset as u64, align) as usize + field_size as usize;
                            } else {
                                offset += align8(field_size) as usize;
                            }
                        }
                        (offsets, types)
                    } else {
                        (
                            (0..fields.len()).map(|i| i * 8).collect(),
                            (0..fields.len()).map(|_| None).collect(),
                        )
                    };

                for (i, (_fname, fexpr)) in fields.iter().enumerate() {
                    let offset = field_offsets.get(i).copied().unwrap_or(i * 8);
                    let field_type = field_types.get(i).and_then(|t| t.clone());

                    if let Some(TypeExpr::FixedArray(count, elem_ty)) = &field_type {
                        let elem_size = type_byte_size(elem_ty);
                        let field_byte_size = count * elem_size;
                        let field_ptr = self.fresh_tmp();
                        self.emit(&format!("    {field_ptr} =l add {ptr}, {offset}"));

                        if let Expr::IntLit(0) = fexpr {
                            self.emit(&format!(
                                "    call $memset(l {field_ptr}, w 0, l {field_byte_size})"
                            ));
                        } else {
                            let (val, _) = self.emit_expr(fexpr, ns)?;
                            self.emit(&format!(
                                "    call $memcpy(l {field_ptr}, l {val}, l {field_byte_size})"
                            ));
                        }
                    } else {
                        let (val, val_ty) = self.emit_expr(fexpr, ns)?;
                        let field_ptr = self.fresh_tmp();
                        self.emit(&format!("    {field_ptr} =l add {ptr}, {offset}"));
                        if val_ty == "l" {
                            self.emit(&format!("    storel {val}, {field_ptr}"));
                        } else {
                            let (ext, _) = self.promote_to_l(val, val_ty);
                            self.emit(&format!("    storel {ext}, {field_ptr}"));
                        }
                    }
                }
                Ok((ptr, "l"))
            }

            Expr::ListLit(elems) => {
                let cap = if elems.is_empty() { 1 } else { elems.len() };
                let hdr_ptr = self.fresh_tmp();
                let buf_ptr = self.fresh_tmp();
                self.emit(&format!("    {hdr_ptr} =l call $malloc(l 24)"));
                self.emit(&format!("    {buf_ptr} =l call $malloc(l {})", cap * 8));

                self.emit(&format!("    storel {buf_ptr}, {hdr_ptr}"));
                let len_slot = self.fresh_tmp();
                let cap_slot = self.fresh_tmp();
                self.emit(&format!("    {len_slot} =l add {hdr_ptr}, 8"));
                self.emit(&format!("    {cap_slot} =l add {hdr_ptr}, 16"));
                self.emit(&format!("    storel {}, {len_slot}", elems.len()));
                self.emit(&format!("    storel {cap}, {cap_slot}"));

                for (i, elem) in elems.iter().enumerate() {
                    let (val, val_ty) = self.emit_expr(elem, ns)?;
                    let off = self.fresh_tmp();
                    let elem_ptr = self.fresh_tmp();
                    self.emit(&format!("    {off} =l mul {}, 8", i));
                    self.emit(&format!("    {elem_ptr} =l add {buf_ptr}, {off}"));
                    if val_ty == "d" {
                        self.emit(&format!("    stored {val}, {elem_ptr}"));
                    } else {
                        let (val_l, _) = self.promote_to_l(val, val_ty);
                        self.emit(&format!("    storel {val_l}, {elem_ptr}"));
                    }
                }

                Ok((hdr_ptr, "l"))
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

                let call_path = {
                    let raw = expr_to_path(callee);
                    expand_alias_path(&raw, &self.type_aliases)
                };
                if let Some(param_types) = self.fn_param_types.get(call_path.as_str()) {
                    let is_variadic = self.variadic_fns.contains_key(fn_name.as_str());
                    let expected = param_types.len();
                    let got = args.len();
                    if !is_variadic && got != expected {
                        return Err(format!(
                            "{}:{}: call to '{}' expects {} argument{}, got {}",
                            self.current_file,
                            line,
                            fn_name,
                            expected,
                            if expected == 1 { "" } else { "s" },
                            got
                        ));
                    }
                }

                if fn_name == "print" || fn_name.ends_with("__print") {
                    let fmt_str = match args.first() {
                        Some(Expr::StrLit(s)) => s.clone(),
                        _ => return Err("print first argument must be a string literal".into()),
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

                if fn_name == "format" || fn_name.ends_with("__format") {
                    let fmt_arg = match args.first() {
                        Some(a) => a,
                        None => return Err("format requires a format string argument".into()),
                    };
                    let fmt_tmp = match fmt_arg {
                        Expr::StrLit(s) => {
                            let rewritten = rewrite_format_string(s);
                            let label = self.intern_string(&rewritten);
                            let t = self.fresh_tmp();
                            self.emit(&format!("    {t} =l copy ${label}"));
                            t
                        }
                        other => {
                            let (raw, _) = self.emit_expr(other, ns)?;
                            let t = self.fresh_tmp();
                            self.emit(&format!("    {t} =l call $rewrite_fmt(l {raw})"));
                            t
                        }
                    };
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
                    let buf = self.fresh_tmp();
                    let size = self.fresh_tmp();
                    self.emit(&format!("    {buf} =l call $malloc(l 4096)"));
                    self.emit(&format!("    {size} =l copy 4096"));
                    let written_w = self.fresh_tmp();
                    if variadic_args.is_empty() {
                        self.emit(&format!(
                            "    {written_w} =w call $snprintf(l {buf}, l {size}, l {fmt_tmp})"
                        ));
                    } else {
                        self.emit(&format!(
                            "    {written_w} =w call $snprintf(l {buf}, l {size}, l {fmt_tmp}, ..., {})",
                            variadic_args.join(", ")
                        ));
                    }
                    let written = self.fresh_tmp();
                    self.emit(&format!("    {written} =l extsw {written_w}"));
                    let copy_size = self.fresh_tmp();
                    self.emit(&format!("    {copy_size} =l add {written}, 1"));
                    let copy = self.fresh_tmp();
                    self.emit(&format!("    {copy} =l call $malloc(l {copy_size})"));
                    self.emit(&format!("    call $memcpy(l {copy}, l {buf}, l {written})"));
                    let nul_ptr = self.fresh_tmp();
                    self.emit(&format!("    {nul_ptr} =l add {copy}, {written}"));
                    self.emit(&format!("    storeb 0, {nul_ptr}"));
                    self.emit(&format!("    call $free(l {buf})"));
                    let hdr = self.fresh_tmp();
                    self.emit(&format!("    {hdr} =l call $malloc(l 24)"));
                    self.emit(&format!("    storel {copy}, {hdr}"));
                    let len_ptr = self.fresh_tmp();
                    self.emit(&format!("    {len_ptr} =l add {hdr}, 8"));
                    self.emit(&format!("    storel {written}, {len_ptr}"));
                    let cap_ptr = self.fresh_tmp();
                    self.emit(&format!("    {cap_ptr} =l add {hdr}, 16"));
                    self.emit(&format!("    storel {copy_size}, {cap_ptr}"));
                    return Ok((hdr, "l"));
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
                        let param_types = self.fn_param_types.get(call_path.as_str()).cloned();
                        let wrapped = if let Some(ref ptypes) = param_types {
                            if let Some(TypeExpr::Named(union_name)) = ptypes.get(i) {
                                if let Some(variants) = self.union_defs.get(union_name).cloned() {
                                    let arg_type_name = match a {
                                        Expr::Ident(n) => {
                                            self.local_type_annotations.get(n).cloned()
                                        }
                                        Expr::IntLit(_) => Some("i32".to_string()),
                                        Expr::FloatLit(_) => Some("f64".to_string()),
                                        Expr::StrLit(_) => Some("ref char".to_string()),
                                        Expr::Bool(_) => Some("bool".to_string()),
                                        Expr::Cast { ty, .. } => {
                                            Some(crate::codegen::type_to_annotation_string(ty))
                                        }
                                        Expr::Call { callee, .. } => {
                                            let path = expr_to_path(callee);
                                            let expanded =
                                                expand_alias_path(&path, &self.module_aliases);
                                            ns.get(&expanded)
                                                .and_then(|qbe_fn| {
                                                    self.fn_ret_type_exprs.get(qbe_fn)
                                                })
                                                .map(type_to_annotation_string)
                                                .filter(|s| !s.is_empty())
                                        }
                                        Expr::Trust(inner) => {
                                            if let Expr::Call { callee, .. } = inner.as_ref() {
                                                let path = expr_to_path(callee);
                                                let expanded =
                                                    expand_alias_path(&path, &self.module_aliases);
                                                ns.get(&expanded)
                                                    .and_then(|qbe_fn| {
                                                        self.fn_ret_type_exprs.get(qbe_fn)
                                                    })
                                                    .map(type_to_annotation_string)
                                                    .filter(|s| !s.is_empty())
                                            } else {
                                                None
                                            }
                                        }
                                        _ => None,
                                    };
                                    let tag = arg_type_name.as_deref().and_then(|atn| {
                                        variants.iter().position(|v| {
                                            matches!(v, TypeExpr::Named(n) if n == atn)
                                                || matches!(v, TypeExpr::Ref(inner) if atn == "ref char" && matches!(inner.as_ref(), TypeExpr::Named(n) if n == "char"))
                                                || matches!(v, TypeExpr::Ref(_) if atn == "ref")
                                                || type_to_annotation_string(v) == atn
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
                        let final_arg = wrapped.unwrap_or_else(|| {
                            // Check if we need to promote w -> l
                            if ty == "w" {
                                if let Some(ref ptypes) = self.fn_param_types.get(call_path.as_str()).cloned()
                                {
                                    if let Some(pt) = ptypes.get(i) {
                                        if qbe_type(pt) == "l" {
                                            let (promoted, _) = self.promote_to_l(tmp, ty);
                                            return format!("l {promoted}");
                                        }
                                    }
                                }
                            }
                            if ty == "l" {
                                if let Some(ref ptypes) = self.fn_param_types.get(call_path.as_str()).cloned()
                                {
                                    if let Some(pt) = ptypes.get(i) {
                                        if qbe_type(pt) == "w" {
                                            let w = self.fresh_tmp();
                                            self.emit(&format!("    {w} =w copy {tmp}"));
                                            return format!("w {w}");
                                        }
                                    }
                                }
                            }
                            format!("{ty} {tmp}")
                        });
                        arg_strs.push(final_arg);
                    }
                }
                let result = self.fresh_tmp();
                let ret_ty = self
                    .fn_ret_types
                    .get(fn_name.as_str())
                    .copied()
                    .unwrap_or("l");
                let args_str = if let Some(&fixed) = self.variadic_fns.get(fn_name.as_str()) {
                    if arg_strs.len() > fixed {
                        let (fixed_args, var_args) = arg_strs.split_at(fixed);
                        if fixed_args.is_empty() {
                            format!("..., {}", var_args.join(", "))
                        } else {
                            format!("{}, ..., {}", fixed_args.join(", "), var_args.join(", "))
                        }
                    } else {
                        arg_strs.join(", ")
                    }
                } else {
                    arg_strs.join(", ")
                };
                if ret_ty.is_empty() {
                    self.emit(&format!("    call ${fn_name}({args_str})"));
                    Ok(("".into(), ""))
                } else {
                    self.emit(&format!(
                        "    {result} ={ret_ty} call ${fn_name}({args_str})"
                    ));
                    Ok((result, ret_ty))
                }
            }

            Expr::BinOp { op, lhs, rhs } => {
                use crate::parser::BinOp::*;

                let lhs_is_result = matches!(lhs.as_ref(), Expr::OkVal(_) | Expr::ErrVal(_));
                let rhs_is_result = matches!(rhs.as_ref(), Expr::OkVal(_) | Expr::ErrVal(_));
                if (lhs_is_result || rhs_is_result) && matches!(op, Eq | Ne) {
                    let result_literal = if lhs_is_result { lhs } else { rhs };
                    let payload_is_float = match result_literal.as_ref() {
                        Expr::OkVal(inner) | Expr::ErrVal(inner) => self.expr_is_float(inner),
                        _ => false,
                    };
                    let (l, _) = self.emit_expr(lhs, ns)?;
                    let (r, _) = self.emit_expr(rhs, ns)?;
                    let result = self.fresh_tmp();
                    let l_tag = self.fresh_tmp();
                    let r_tag = self.fresh_tmp();
                    self.emit(&format!("    {l_tag} =l loadl {l}"));
                    self.emit(&format!("    {r_tag} =l loadl {r}"));
                    let tags_eq = self.fresh_tmp();
                    self.emit(&format!("    {tags_eq} =w ceql {l_tag}, {r_tag}"));
                    let l_val_ptr = self.fresh_tmp();
                    let r_val_ptr = self.fresh_tmp();
                    self.emit(&format!("    {l_val_ptr} =l add {l}, 8"));
                    self.emit(&format!("    {r_val_ptr} =l add {r}, 8"));
                    let vals_eq = self.fresh_tmp();
                    if payload_is_float {
                        let l_val = self.fresh_tmp();
                        let r_val = self.fresh_tmp();
                        self.emit(&format!("    {l_val} =d loadd {l_val_ptr}"));
                        self.emit(&format!("    {r_val} =d loadd {r_val_ptr}"));
                        self.emit(&format!("    {vals_eq} =w ceqd {l_val}, {r_val}"));
                    } else {
                        let l_val = self.fresh_tmp();
                        let r_val = self.fresh_tmp();
                        self.emit(&format!("    {l_val} =l loadl {l_val_ptr}"));
                        self.emit(&format!("    {r_val} =l loadl {r_val_ptr}"));
                        self.emit(&format!("    {vals_eq} =w ceql {l_val}, {r_val}"));
                    }
                    let both = self.fresh_tmp();
                    self.emit(&format!("    {both} =w and {tags_eq}, {vals_eq}"));
                    if matches!(op, Ne) {
                        self.emit(&format!("    {result} =w ceqw {both}, 0"));
                    } else {
                        self.emit(&format!("    {result} =w copy {both}"));
                    }
                    return Ok((result, "w"));
                }

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
                        BitXor => format!("{tmp} =l xor {l}, {r}"),
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
                        BitXor => format!("{tmp} =w xor {l}, {r}"),
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
                            let slot = self
                                .locals
                                .get(name)
                                .cloned()
                                .ok_or_else(|| format!("{}: in fn '{}': undefined variable in addr-of: '{name}'", self.current_file, self.current_fn))?;
                            // Check if this slot holds a struct type (stored as a pointer).
                            // If so, load the pointer from the slot; otherwise return the slot address.
                            let is_struct_slot = self
                                .local_type_annotations
                                .get(name)
                                .map(|ann| {
                                    let type_name = ann.strip_prefix("ref ").unwrap_or(ann);
                                    self.type_defs.contains_key(type_name)
                                        || self.extern_type_defs.contains_key(type_name)
                                })
                                .unwrap_or(false);
                            if is_struct_slot {
                                self.emit(&format!("    {tmp} =l loadl {slot}"));
                            } else {
                                self.emit(&format!("    {tmp} =l copy {slot}"));
                            }
                            return Ok((tmp, "l"));
                        }
                        Err(format!(
                            "cannot take address of immutable binding '{name}' — declare it as 'mut' or use a function name"
                        ))
                    }
                    other => {
                        if matches!(other, Expr::Field(_, _)) {
                            let field_ptr = self.emit_field_ptr(other, ns)?;
                            self.emit(&format!("    {tmp} =l copy {field_ptr}"));
                            return Ok((tmp, "l"));
                        }
                        let path = expr_to_path(other);
                        let expanded = expand_alias_path(&path, &self.type_aliases.clone());
                        if let Some(qbe_name) = ns.get(&expanded) {
                            self.emit(&format!("    {tmp} =l copy ${qbe_name}"));
                            Ok((tmp, "l"))
                        } else {
                            Err(format!(
                                "addr(): address-of operator only supports function names or mutable variables, got {other:?}"
                            ))
                        }
                    }
                }
            }

            Expr::ZeroInit(type_name) => {
                let size = if let Some(field_defs) = self
                    .type_defs
                    .get(type_name.as_str())
                    .or_else(|| self.extern_type_defs.get(type_name.as_str()))
                {
                    let has_fixed_arrays = field_defs
                        .iter()
                        .any(|f| matches!(f.ty, TypeExpr::FixedArray(_, _)));
                    let mut total: usize = 0;
                    for field in field_defs {
                        let field_size = type_byte_size(&field.ty);
                        if has_fixed_arrays {
                            let align = type_alignment(&field.ty);
                            total = align_to(total as u64, align) as usize + field_size as usize;
                        } else {
                            total += align8(field_size) as usize;
                        }
                    }
                    total
                } else {
                    return Err(format!("unknown type '{type_name}' in zero-init"));
                };
                let ptr = self.fresh_tmp();
                self.emit(&format!("    {ptr} =l alloc8 {size}"));
                if size > 0 {
                    self.emit(&format!("    call $memset(l {ptr}, w 0, l {size})"));
                }
                Ok((ptr, "l"))
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
    matches!(
        stmts.last(),
        Some(Stmt::Return(_)) | Some(Stmt::Break) | Some(Stmt::Continue)
    )
}

/// Recursively checks whether a block consists entirely of "trusted" operations.
fn all_trusted_stmts(stmts: &[Stmt]) -> bool {
    stmts.iter().all(|s| match s {
        Stmt::Expr(Expr::Trust(_)) => true,
        Stmt::Expr(Expr::Call { callee, .. }) => !expr_to_path(callee).is_empty(),
        Stmt::Expr(Expr::Builtin { .. }) => true,
        Stmt::Pre(_) | Stmt::Post(_) | Stmt::GuardedPre(_) | Stmt::GuardedPost(_) => true,
        Stmt::Val { .. }
        | Stmt::Return(_)
        | Stmt::Break
        | Stmt::Continue
        | Stmt::Increment(_)
        | Stmt::Decrement(_)
        | Stmt::AddAssign(..)
        | Stmt::SubAssign(..) => true,
        Stmt::Assert(..) => true,
        Stmt::Assign { target, .. } => matches!(target, Expr::Deref(_) | Expr::Trust(_)),
        Stmt::Defer(body) => all_trusted_stmts(body),
        Stmt::If {
            then, elif_, else_, ..
        } => {
            all_trusted_stmts(then)
                && elif_.iter().all(|(_, b)| all_trusted_stmts(b))
                && else_.as_deref().map_or(true, all_trusted_stmts)
        }
        Stmt::For { body, .. } => all_trusted_stmts(body),
        Stmt::ForIn { body, .. } => all_trusted_stmts(body),
        Stmt::Match {
            some_body,
            none_body,
            ..
        } => all_trusted_stmts(some_body) && all_trusted_stmts(none_body),
        Stmt::MatchResult {
            ok_body, err_body, ..
        } => all_trusted_stmts(ok_body) && all_trusted_stmts(err_body),
        Stmt::MatchEnum { arms, .. } => arms.iter().all(|(_, b)| all_trusted_stmts(b)),
        Stmt::MatchUnion {
            arms, else_body, ..
        } => {
            arms.iter().all(|(_, b)| all_trusted_stmts(b))
                && else_body.as_deref().map_or(true, all_trusted_stmts)
        }
        Stmt::MatchString {
            arms, else_body, ..
        } => {
            arms.iter().all(|(_, b)| all_trusted_stmts(b))
                && else_body.as_deref().map_or(true, all_trusted_stmts)
        }
        Stmt::When { body, .. } => all_trusted_stmts(body),
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
        Stmt::GuardedPre(contracts) | Stmt::GuardedPost(contracts) => contracts
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
        Stmt::ForIn { iterable, body, .. } => {
            find_bare_builtin_in_expr(iterable).or_else(|| find_bare_builtin_in_stmts(body))
        }
        Stmt::Match {
            expr,
            some_body,
            none_body,
            ..
        } => find_bare_builtin_in_expr(expr)
            .or_else(|| find_bare_builtin_in_stmts(some_body))
            .or_else(|| find_bare_builtin_in_stmts(none_body)),
        Stmt::MatchResult {
            expr,
            ok_body,
            err_body,
            ..
        } => find_bare_builtin_in_expr(expr)
            .or_else(|| find_bare_builtin_in_stmts(ok_body))
            .or_else(|| find_bare_builtin_in_stmts(err_body)),
        Stmt::MatchEnum { expr, arms } => find_bare_builtin_in_expr(expr)
            .or_else(|| arms.iter().find_map(|(_, b)| find_bare_builtin_in_stmts(b))),
        Stmt::MatchUnion {
            expr,
            arms,
            else_body,
        } => find_bare_builtin_in_expr(expr)
            .or_else(|| arms.iter().find_map(|(_, b)| find_bare_builtin_in_stmts(b)))
            .or_else(|| else_body.as_deref().and_then(find_bare_builtin_in_stmts)),
        Stmt::MatchString {
            expr,
            arms,
            else_body,
        } => find_bare_builtin_in_expr(expr)
            .or_else(|| arms.iter().find_map(|(_, b)| find_bare_builtin_in_stmts(b)))
            .or_else(|| else_body.as_deref().and_then(find_bare_builtin_in_stmts)),
        Stmt::When { body, .. } => find_bare_builtin_in_stmts(body),
        Stmt::Increment(_)
        | Stmt::Decrement(_)
        | Stmt::AddAssign(..)
        | Stmt::SubAssign(..)
        | Stmt::Break
        | Stmt::Continue => None,
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
        | Expr::OkVal(expr)
        | Expr::ErrVal(expr)
        | Expr::Try(expr)
        | Expr::Addr(expr)
        | Expr::Deref(expr) => find_bare_builtin_in_expr(expr),
        Expr::Field(base, _) => find_bare_builtin_in_expr(base),
        Expr::StructLit { fields } => fields
            .iter()
            .find_map(|(_, e)| find_bare_builtin_in_expr(e)),
        Expr::ListLit(elems) => elems.iter().find_map(find_bare_builtin_in_expr),
        Expr::ArgsPack(exprs) => exprs.iter().find_map(find_bare_builtin_in_expr),
        Expr::IntLit(_)
        | Expr::FloatLit(_)
        | Expr::StrLit(_)
        | Expr::Ident(_)
        | Expr::Bool(_)
        | Expr::None => None,
        Expr::ZeroInit(_) => None,
    }
}

/// Returns true if any statement is a bare (non-trusted) deref-assign: `deref(...) = ...`
fn find_bare_deref_assign_in_stmts(stmts: &[Stmt]) -> bool {
    stmts.iter().any(|s| find_bare_deref_assign_in_stmt(s))
}

fn find_bare_deref_assign_in_stmt(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Assign { target, .. } => matches!(target, Expr::Deref(_)),
        Stmt::If {
            then, elif_, else_, ..
        } => {
            find_bare_deref_assign_in_stmts(then)
                || elif_
                    .iter()
                    .any(|(_, b)| find_bare_deref_assign_in_stmts(b))
                || else_
                    .as_deref()
                    .map_or(false, find_bare_deref_assign_in_stmts)
        }
        Stmt::For { body, .. } => find_bare_deref_assign_in_stmts(body),
        Stmt::ForIn { body, .. } => find_bare_deref_assign_in_stmts(body),
        Stmt::Match {
            some_body,
            none_body,
            ..
        } => {
            find_bare_deref_assign_in_stmts(some_body) || find_bare_deref_assign_in_stmts(none_body)
        }
        Stmt::MatchResult {
            ok_body, err_body, ..
        } => find_bare_deref_assign_in_stmts(ok_body) || find_bare_deref_assign_in_stmts(err_body),
        Stmt::MatchEnum { arms, .. } => {
            arms.iter().any(|(_, b)| find_bare_deref_assign_in_stmts(b))
        }
        Stmt::MatchUnion {
            arms, else_body, ..
        } => {
            arms.iter().any(|(_, b)| find_bare_deref_assign_in_stmts(b))
                || else_body
                    .as_deref()
                    .map_or(false, find_bare_deref_assign_in_stmts)
        }
        Stmt::Defer(body) | Stmt::When { body, .. } => find_bare_deref_assign_in_stmts(body),
        _ => false,
    }
}

fn build_namespace(resolved: &ResolvedModule) -> Namespace {
    let mut ns = HashMap::new();
    collect_ns(resolved, "", &mut ns);
    ns
}

fn build_cross_module_consts(
    resolved: &ResolvedModule,
) -> HashMap<String, (String, &'static str)> {
    let mut map = HashMap::new();
    collect_cross_module_consts(resolved, "", &mut map);
    map
}

fn collect_cross_module_consts(
    module: &ResolvedModule,
    prefix: &str,
    map: &mut HashMap<String, (String, &'static str)>,
) {
    for item in &module.ast.items {
        if let Item::Const { name, expr, public, .. } = item {
            if !prefix.is_empty() && !public {
                continue;
            }
            let val_ty: Option<(String, &'static str)> = match expr {
                crate::parser::Expr::IntLit(n) => Some((n.to_string(), "l")),
                crate::parser::Expr::FloatLit(f) => Some((format!("d_{f}"), "d")),
                crate::parser::Expr::Bool(b) => {
                    Some((if *b { "1" } else { "0" }.to_string(), "w"))
                }
                crate::parser::Expr::UnOp {
                    op: crate::parser::UnOp::Neg,
                    expr,
                } => match expr.as_ref() {
                    crate::parser::Expr::IntLit(n) => Some((format!("-{n}"), "l")),
                    crate::parser::Expr::FloatLit(f) => Some((format!("d_-{f}"), "d")),
                    _ => None,
                },
                _ => None,
            };
            if let Some(val_ty) = val_ty {
                let key = if prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{prefix}.{name}")
                };
                map.insert(key, val_ty);
            }
        }
    }
    for (import_name, child) in &module.imports {
        let child_prefix = if prefix.is_empty() {
            import_name.clone()
        } else {
            format!("{prefix}.{import_name}")
        };
        collect_cross_module_consts(child, &child_prefix, map);
    }
}

fn build_ret_types(resolved: &ResolvedModule) -> HashMap<String, &'static str> {
    let mut map = HashMap::new();
    collect_ret_types(resolved, &mut map);
    map
}

fn build_ret_type_exprs(resolved: &ResolvedModule) -> HashMap<String, TypeExpr> {
    let mut map = HashMap::new();
    collect_ret_type_exprs(resolved, &mut map);
    map
}

fn collect_ret_type_exprs(module: &ResolvedModule, map: &mut HashMap<String, TypeExpr>) {
    collect_ret_type_exprs_prefixed(module, "", map);
}

fn collect_ret_type_exprs_prefixed(
    module: &ResolvedModule,
    prefix: &str,
    map: &mut HashMap<String, TypeExpr>,
) {
    for item in &module.ast.items {
        if let Item::Fn(f) = item {
            map.insert(fn_qbe_name_prefixed(f, prefix), f.ret.clone());
        }
    }
    for (import_name, child) in &module.imports {
        let child_prefix = if prefix.is_empty() {
            import_name.clone()
        } else {
            format!("{prefix}__{import_name}")
        };
        collect_ret_type_exprs_prefixed(child, &child_prefix, map);
    }
}

fn collect_ret_types(module: &ResolvedModule, map: &mut HashMap<String, &'static str>) {
    collect_ret_types_prefixed(module, "", map);
}

fn collect_ret_types_prefixed(
    module: &ResolvedModule,
    prefix: &str,
    map: &mut HashMap<String, &'static str>,
) {
    for item in &module.ast.items {
        match item {
            Item::Fn(f) => {
                map.insert(fn_qbe_name_prefixed(f, prefix), qbe_type(&f.ret));
            }
            Item::ExternFn { symbol, ret, .. } => {
                map.insert(symbol.clone(), qbe_type(ret));
            }
            _ => {}
        }
    }
    for (import_name, child) in &module.imports {
        let child_prefix = if prefix.is_empty() {
            import_name.clone()
        } else {
            format!("{prefix}__{import_name}")
        };
        collect_ret_types_prefixed(child, &child_prefix, map);
    }
}

fn build_param_types(resolved: &ResolvedModule) -> HashMap<String, Vec<TypeExpr>> {
    let mut map = HashMap::new();
    collect_param_types(resolved, &mut map);
    map
}

fn collect_param_types(module: &ResolvedModule, map: &mut HashMap<String, Vec<TypeExpr>>) {
    collect_param_types_with_prefix(module, "", map);
}

fn collect_param_types_with_prefix(
    module: &ResolvedModule,
    prefix: &str,
    map: &mut HashMap<String, Vec<TypeExpr>>,
) {
    for item in &module.ast.items {
        match item {
            Item::Fn(f) => {
                let local_key = match &f.namespace {
                    Some(type_name) => format!("{type_name}.{}", f.name),
                    None => f.name.clone(),
                };
                let key = if prefix.is_empty() {
                    local_key
                } else {
                    format!("{prefix}.{local_key}")
                };
                map.insert(key, f.params.iter().map(|(_, ty)| ty.clone()).collect());
            }
            Item::ExternFn { symbol, params, .. } => {
                map.insert(
                    symbol.clone(),
                    params.iter().map(|(_, ty)| ty.clone()).collect(),
                );
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
        collect_param_types_with_prefix(child, &child_prefix, map);
    }
}

fn build_variadic_set(resolved: &ResolvedModule) -> HashMap<String, usize> {
    let mut map = HashMap::new();
    collect_variadic(resolved, &mut map);
    map
}

fn collect_variadic(module: &ResolvedModule, map: &mut HashMap<String, usize>) {
    for item in &module.ast.items {
        if let Item::ExternFn {
            symbol,
            variadic_after: Some(n),
            ..
        } = item
        {
            map.insert(symbol.clone(), *n);
        }
    }
    for child in module.imports.values() {
        collect_variadic(child, map);
    }
}

fn build_result_void_ok(resolved: &ResolvedModule) -> std::collections::HashSet<String> {
    let mut set = std::collections::HashSet::new();
    collect_result_void_ok(resolved, &mut set);
    set
}

fn collect_result_void_ok(module: &ResolvedModule, set: &mut std::collections::HashSet<String>) {
    collect_result_void_ok_prefixed(module, "", set);
}

fn collect_result_void_ok_prefixed(
    module: &ResolvedModule,
    prefix: &str,
    set: &mut std::collections::HashSet<String>,
) {
    for item in &module.ast.items {
        match item {
            Item::Fn(f) => {
                if is_result_void_ok(&f.ret) {
                    set.insert(fn_qbe_name_prefixed(f, prefix));
                }
            }
            Item::ExternFn { symbol, ret, .. } => {
                if is_result_void_ok(ret) {
                    set.insert(symbol.clone());
                }
            }
            _ => {}
        }
    }
    for (import_name, child) in &module.imports {
        let child_prefix = if prefix.is_empty() {
            import_name.clone()
        } else {
            format!("{prefix}__{import_name}")
        };
        collect_result_void_ok_prefixed(child, &child_prefix, set);
    }
}

/// Check if a TypeExpr is result[void, ...]
fn is_result_void_ok(ty: &TypeExpr) -> bool {
    if let TypeExpr::Result(ok_ty, _) = ty {
        matches!(ok_ty.as_ref(), TypeExpr::Void)
    } else {
        false
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
                let key = if prefix.is_empty() {
                    local_key
                } else {
                    format!("{prefix}.{local_key}")
                };
                trusted.insert(key);
            }
            Item::ExternFn { name, symbol, .. } => {
                let key = if prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{prefix}.{name}")
                };
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

fn fn_qbe_name(f: &FnDef) -> String {
    fn_qbe_name_prefixed(f, "")
}

fn fn_qbe_name_prefixed(f: &FnDef, module_prefix: &str) -> String {
    let base = match &f.namespace {
        Some(type_name) => format!("{}__{}", type_name, f.name),
        None => f.name.clone(),
    };
    if module_prefix.is_empty() || base == "main" {
        base
    } else {
        format!("{module_prefix}__{base}")
    }
}

/// Symbols emitted directly by builtins or the runtime.
const RESERVED_SYMBOLS: &[&str] = &[
    "malloc",
    "realloc",
    "free",
    "memset",
    "memcpy",
    "printf",
    "fprintf",
    "dprintf",
    "snprintf",
    "puts",
    "fputs",
    "fgets",
    "fflush",
    "fdopen",
    "fopen",
    "fclose",
    "getchar",
    "abort",
    "strlen",
    "strcmp",
    "gettimeofday",
    "arc4random_buf",
    "arc4random_uniform",
    "sx_stdout",
    "sx_stderr",
    "sx_stdin",
];

fn collect_ns(module: &ResolvedModule, prefix: &str, ns: &mut Namespace) {
    collect_ns_with_qbe_prefix(module, prefix, prefix, ns);
}

fn collect_ns_with_qbe_prefix(
    module: &ResolvedModule,
    prefix: &str,
    qbe_prefix: &str,
    ns: &mut Namespace,
) {
    println!("collect_ns: prefix='{}' file='{}' imports={:?}", prefix, module.filename, module.imports.keys().collect::<Vec<_>>());
    for item in &module.ast.items {
        match item {
            Item::Fn(f) => {
                if f.namespace.is_some() && !f.public {
                    continue;
                }
                let qbe = fn_qbe_name_prefixed(f, qbe_prefix);
                let local_key = match &f.namespace {
                    Some(type_name) => format!("{type_name}.{}", f.name),
                    None => f.name.clone(),
                };
                let key = if prefix.is_empty() {
                    local_key
                } else {
                    format!("{prefix}.{local_key}")
                };
                println!("  ns insert: key='{}' qbe='{}'", key, qbe);
                ns.insert(key, qbe);
            }
            Item::ExternFn {
                name,
                symbol,
                public,
                ..
            } => {
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
        let child_qbe_prefix = if qbe_prefix.is_empty() {
            import_name.clone()
        } else {
            format!("{qbe_prefix}__{import_name}")
        };
        collect_ns_with_qbe_prefix(child, &child_prefix, &child_qbe_prefix, ns);
    }
}

fn resolve_call_name(
    callee: &Expr,
    ns: &Namespace,
    aliases: &HashMap<String, String>,
) -> Result<String, String> {
    let path = expr_to_path(callee);
    let expanded = expand_alias_path(&path, aliases);
    if path.contains("collections") {
        println!("resolve_call_name: path='{}' expanded='{}'", path, expanded);
    }
    ns.get(&expanded)
        .cloned()
        .ok_or_else(|| {
            println!("FAILED to resolve '{}'. Namespace keys: {:?}", expanded, ns.keys().collect::<Vec<_>>());
            format!("unknown function: {path}")
        })
}

fn expr_to_path(expr: &Expr) -> String {
    match expr {
        Expr::Ident(name) => name.clone(),
        Expr::Field(base, field) => format!("{}.{field}", expr_to_path(base)),
        _ => String::new(),
    }
}

/// Expand the leading segment of a dotted path through the alias map.
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
        (TypeExpr::FixedArray(a_count, a_elem), TypeExpr::FixedArray(b_count, b_elem)) => {
            a_count == b_count && type_expr_matches(a_elem, b_elem)
        }
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
