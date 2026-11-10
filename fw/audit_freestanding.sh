#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

python3 - <<'PY'
import pathlib, sys

FORBIDDEN = {
    "malloc","calloc","realloc","free","memcpy","memmove","memset","memcmp",
    "strlen","strcmp","strncmp","strcpy","strcat","strchr","strrchr","strstr",
    "strdup","strtok_r","atoi","strtol","isdigit","toupper","tolower",
    "printf","fprintf","dprintf","snprintf","puts","fputs",
    "fopen","fclose","fread","fwrite","fseek","ftell","rewind","fflush",
    "fgetc","getchar","fputc","putchar","fgets","fdopen","fileno","feof","ferror",
    "getenv","getcwd","chdir","access","mkdir","rmdir","remove","rename","unlink",
    "realpath","readlink","isatty","stat","lstat","fstat","opendir","closedir","readdir",
    "exit","system","popen","pclose","gettimeofday","localtime","usleep","sleep","time",
    "getrandom","arc4random_buf","pthread_create","pthread_join","pthread_detach",
    "pthread_self","sched_yield","pthread_mutex_init","pthread_mutex_destroy",
    "pthread_mutex_lock","pthread_mutex_unlock","socket","bind","listen","accept",
    "connect","send","recv","sendto","recvfrom","setsockopt","getsockopt","poll",
    "htons","ntohs","closesocket","signal","raise",
}

SKIP = {
    "spectrelib/json/core.sx",
    "spectrelib/net/http.sx",
    "spectrelib/runtime/core.sx",
}

def strip_comment(line):
    return line.split("//", 1)[0]

errors = []
for path in sorted(pathlib.Path("spectrelib").rglob("*.sx")):
    rel = str(path)
    if rel in SKIP:
        continue
    stack = []
    for lineno, raw in enumerate(path.read_text().splitlines(), 1):
        line = strip_comment(raw)
        s = line.strip()
        if s.startswith("when "):
            stack.append(("when", s[5:].split("{",1)[0].strip()))
        elif s.startswith("or when "):
            stack.append(("when", s[8:].split("{",1)[0].strip()))
        elif s.startswith("} otherwise"):
            if stack:
                stack.pop()
            stack.append(("when", "otherwise"))
        elif s.startswith("otherwise"):
            stack.append(("when", "otherwise"))
        elif s.startswith("}"):
            if stack:
                stack.pop()
        skip = any(kind == "when" and (
            sel in ("windows", "darwin", "otherwise", "hosted")
        ) for kind, sel in stack)
        if skip or not s.startswith("extern (C"):
            continue
        if " fn " not in s:
            continue
        try:
            tail = s.split(" fn ",1)[1]
            name = tail.split("(",1)[0].strip()
        except Exception:
            continue
        symbol = ""
        if '= "' in s:
            symbol = s.rsplit('= "', 1)[1].split('"', 1)[0]
        if symbol.startswith(("sx_", "__sx_", "sxqbe_")):
            continue
        if name in FORBIDDEN or symbol in FORBIDDEN:
            errors.append(f"{rel}:{lineno}: host libc extern '{name}' -> '{symbol}' in freestanding source")

if errors:
    print("freestanding runtime audit failed:", file=sys.stderr)
    for e in errors:
        print("  " + e, file=sys.stderr)
    sys.exit(1)
print("freestanding runtime audit: OK")
PY
