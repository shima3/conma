#!/usr/bin/env python3
"""Sum the steps of ConSh (Rust VM, CONMA_PROFILE) by role, as in the table 1.1 of
docs/consh/dev/COMPILE2-PLAN-ja.md.

    CONSH_IMPL=conma CONMA_PROFILE=/tmp/p.tsv consh/bin/consh SCRIPT
    consh/bench/roles.py /tmp/p.tsv [N]

With N (the number of iterations of the script), the steps are also shown per
iteration. The roles are told by the names of the functions (vm/tools/vm_profile.py
gives the steps of each); a function that fits none is under "ほか".
"""
import os
import re, sys, subprocess, collections
cats = [
 ("変数", r"^(Sh_lookup|Var_fields|Value_force|Var_def_value|Env_lookup|Assign_compile|Def_value_code|Sh_defined|Bvar_args|Ref_part|Sjs_copy_into|Env_push)$"),
 ("$( ) の値", r"^(__LazyList_|Subst_lines|__Lines_|Lazy_drain|Lazy_ahead|Chan_|__Chan|Input_|Stdin_|Line_chomp|Lines_chomped)"),
 ("文の終わりの $( ) 待ち", r"^(Subst_settle|Substs_none_done|Subst_wait)"),
 ("ループ", r"^(For_|Loop_|Eval_for|For$)"),
 ("関数の呼び出し", r"^(Func_|Arity_|Cmd_run_resolved|Seg_register|Eval_return)"),
 ("コマンドの語の展開と準備", r"^(Eval_cmd|Cmd_|Arg_|Eval_args|Expand_|Parts_|Part_|Glob_|List_|Fields_|Args_compile|Literal_|Word_|Single_|Limit_|Values_leaves|Node_split|Value_head|Builtin_lookup|Builtin_)"),
 ("リダイレクトと出力の受け取り", r"^(Capture_|Eval_redirs|Redirs_|Redir_|Target_|Open_|Echo_|Bg_spec|Func_targets|Child_target)"),
 ("文を順に実行", r"^(Eval_lines|Code_chain|Sh_set_status|Stmt_|Lines_|Eval_stmt|Sh_status)"),
 ("if 文", r"^(If_|Eval_if)"),
]
def cat(name):
    for c, rx in cats:
        if re.search(rx, name): return c
    return "ほか"
path = sys.argv[1]
iters = int(sys.argv[2]) if len(sys.argv) > 2 else 1
root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
out = subprocess.run([sys.executable, os.path.join(root, "vm/tools/vm_profile.py"), "--top", "5000", path],capture_output=True,text=True).stdout
tot = int(re.search(r"steps: ([\d,]+)", out).group(1).replace(",",""))
sec = out.split("== functions")[1].split("== primitives")[0]
agg = collections.Counter(); oth = collections.Counter()
for l in sec.split("\n"):
    m = re.match(r"\s*([\d,]+)\s+[\d.]+%\s+([\d,]+)\s+\S+\s+(\S+)", l)
    if not m: continue
    st = int(m.group(1).replace(",","")); fn = m.group(3)
    c = cat(fn); agg[c] += st
    if c == "ほか": oth[fn] += st
print(f"steps {tot}" + (f", {tot/iters:.0f} per iteration" if iters > 1 else ""))
for c, v in agg.most_common():
    print(f"{c}\t{v/iters:.0f}\t{100*v/tot:.0f}%")
print("ほか:", ", ".join(f"{k} {v/iters:.0f}" for k, v in oth.most_common(8)))
