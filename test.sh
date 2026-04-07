cargo test
./test_stdlib.sh
./target/release/spectre self/parser.sx --test
./target/release/spectre self/lexer.sx --test
./target/release/spectre self/codegen.sx --test
./target/release/spectre self/sema.sx --test
