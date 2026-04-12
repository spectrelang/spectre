- string-to-int and int-to-string conv      DONE
- string-to-float and float-to-string conv  DONE
- dynamic arrays of structs                 DONE
- error propagation syntax                  DONE
- tagged union pattern matching on structs  DONE
- file/dir iteration in fs.sx               DONE
- proper maps with string keys              DONE

 self-hosted codegen: match binding type inference for result/option

     When matching `result[T, E]` or `option[T]`, the inner type must be registered for the
     binding so that field access (e.g. `n.d0`) computes correct byte offsets. Without this,
     `gen_field` can't find the struct definition for the binding, defaults to offset 0,
     and loads the `kind` field (value 64) instead of `d0` (offset 8) and `d1` (offset 16).
     Reference: `src/codegen.rs` lines ~1903-1990 where the reference compiler parses
     `result[...` / `option[...` annotations and registers them in `local_type_annotations`.
