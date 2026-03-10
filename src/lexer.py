import sys

def tokenize(source_code):
    tokens = []
    i = 0
    line_num = 1
    line_start = 0

    while i < len(source_code):
        c = source_code[i]

        # ブロックコメント（ネスト対応）
        if source_code[i:i+2] == '#|':
            depth = 1
            i += 2
            while i < len(source_code) - 1 and depth > 0:
                if source_code[i:i+2] == '#|':
                    depth += 1
                    i += 2
                elif source_code[i:i+2] == '|#':
                    depth -= 1
                    i += 2
                else:
                    if source_code[i] == '\n':
                        line_num += 1
                        line_start = i + 1
                    i += 1
            if depth != 0:
                print(f"Error: Unterminated block comment", file=sys.stderr)
            continue

        # 行コメント
        if c == ';':
            while i < len(source_code) and source_code[i] != '\n':
                i += 1
            continue

        # 空白
        if c in ' \t\r':
            i += 1
            continue

        # 改行
        if c == '\n':
            line_num += 1
            line_start = i + 1
            i += 1
            continue

        col = i - line_start + 1

        # 文字列
        if c == '"':
            j = i + 1
            while j < len(source_code):
                if source_code[j] == '\\':
                    j += 2
                elif source_code[j] == '"':
                    j += 1
                    break
                else:
                    j += 1
            print(f"{line_num}\t{col}\tSTRING\t{source_code[i:j]}")
            i = j
            continue

        if c == '(':
            print(f"{line_num}\t{col}\tLPAREN\t(")
            i += 1; continue
        if c == ')':
            print(f"{line_num}\t{col}\tRPAREN\t)")
            i += 1; continue
        if c == ',':
            print(f"{line_num}\t{col}\tCOMMA\t,")
            i += 1; continue

        # Symbol
        symbol_chars = set('abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_!?*+-/=<>$%&|.^')
        if c in symbol_chars:
            j = i
            while j < len(source_code) and source_code[j] in symbol_chars:
                j += 1
            print(f"{line_num}\t{col}\tSYMBOL\t{source_code[i:j]}")
            i = j
            continue

        print(f"Error: Unexpected character '{c}' at line {line_num}, column {col}", file=sys.stderr)
        i += 1

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python lexer.py <source_file>")
        sys.exit(1)
    with open(sys.argv[1], 'r', encoding='utf-8') as f:
        tokenize(f.read())
