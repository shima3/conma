#!/usr/bin/env python3
"""vproc_step.py — Execute one Operator Application of a ConMa VProc."""

import sys, re
import types

class VProcError(Exception): pass

def sinfo_to_str(sinfo):
    """Format SInfo as 'filename:line' for error messages."""
    if not sinfo:
        return None
    parts = [unquote(s) for s in sinfo if isinstance(s, str)]
    if len(parts) >= 2:
        return f"{parts[0]}:{parts[1]}"
    if len(parts) == 1:
        return parts[0]
    return None

def format_error(msg, sinfo):
    """Prefix msg with source location from sinfo if available."""
    loc = sinfo_to_str(sinfo)
    return f"{loc}: {msg}" if loc else msg


# ── S-expression ──────────────────────────────────────────────────────────────

def tokenize(text):
    return re.findall(r'"(?:[^"\\]|\\.)*"|\(|\)|[^\s()]+', text)

def parse_tokens(tokens):
    if not tokens: raise VProcError("EOF")
    tok = tokens.pop(0)
    if tok == '(':
        lst = []
        while tokens[0] != ')': lst.append(parse_tokens(tokens))
        tokens.pop(0); return lst
    if tok == ')': raise VProcError("unexpected )")
    return tok

def parse_sexp(text): return parse_tokens(tokenize(text))

def sexp_to_str(x, indent=0):
    if not isinstance(x, list): return str(x)
    if not x: return "()"
    inline = "(" + " ".join(sexp_to_str(i) for i in x) + ")"
    if len(inline) <= 80: return inline
    pad = "  " * (indent + 1)
    inner = ("\n" + pad).join(sexp_to_str(i, indent + 1) for i in x)
    return "(" + inner + ")"

# ── Values ────────────────────────────────────────────────────────────────────

class _Null:
    def __repr__(self): return "null"
NULL = _Null()

def unquote(s):
    if isinstance(s, str) and len(s) >= 2 and s[0] == s[-1] == '"': return s[1:-1]
    return s

def quoted(s):
    if isinstance(s, str) and not (s.startswith('"') and s.endswith('"')): return f'"{s}"'
    return s

class Closure:
    def __init__(self, params, body, lvenv):
        self.params, self.body, self.lvenv = list(params), body, list(lvenv)
    def __repr__(self): return f"Closure(params={self.params})"

class SeqClosure:
    def __init__(self, olist, sinfo, lvenv):
        self.params, self.olist, self.sinfo, self.lvenv = ["__Sink__"], list(olist), sinfo, list(lvenv)
    def __repr__(self): return f"SeqClosure({self.olist})"

class CFrame:
    def __init__(self, closure, sinfo): self.closure, self.sinfo = closure, sinfo
class Stream:
    """Wraps a raw OS file descriptor as a ConMa value."""
    def __init__(self, fd, mode):
        self.fd   = fd                  # raw file descriptor
        self.mode = mode                # "r" or "w"
        self._closed = False
    def close(self):
        if not self._closed:
            import os
            os.close(self.fd)
            self._closed = True
    def value_to_sexp(self):
        return ["__OSStream__", f'"{self.mode}:{id(self)}"']
    def __repr__(self):
        return f"Stream({self.mode}, fd={self.fd})"
class Number:
    """An immutable numeric value (int or float) as a ConMa value."""
    def __init__(self, value):
        self.value = value   # Python int or float
    def value_to_sexp(self):
        return ["__Number__", str(self.value)]
    def __repr__(self):
        return f"Number({self.value!r})"
class LList:
    """A cons cell representing a node in a linked list (also used for CChain)."""
    def __init__(self, head, tail):
        self.head = head  # any ConMa value (CFrame when used as CChain)
        self.tail = tail  # LList or NULL
    def value_to_sexp(self):
        return ["__LList__", value_to_sexp(self.head), value_to_sexp(self.tail)]
    def __repr__(self):
        return f"LList({self.head!r}, {self.tail!r})"




def is_null_value(v):
    return v is NULL or (isinstance(v, list) and v and v[0] == "Null")

def is_prim_func(v):
    return isinstance(v, list) and v[0] == "PrimFunc"

# ── Module ────────────────────────────────────────────────────────────────────

def closure_from_function(func_ast, lvenv):
    head_node = body_node = None
    for child in func_ast[2:]:
        if isinstance(child, list):
            if child[0] == "Head": head_node = child
            elif child[0] == "Body": body_node = child
    params = [unquote(c[2]) for c in (head_node[2:] if head_node else [])
              if isinstance(c, list) and c[0] == "Variable"]
    return Closure(params, body_node, lvenv)

def load_module(module_sexp):
    gvenv = {}
    for item in module_sexp[2:]:
        if not isinstance(item, list) or item[0] != "G": continue
        gid = int(item[1])
        if len(item) < 4 or item[3] == "Undefined":
            # gvenv[gid] = None
            gvenv[gid] = PRIMITIVES.get(unquote(item[2]))
        else:
            val = item[3]
            if isinstance(val, list) and val[0] == "Function":
                gvenv[gid] = closure_from_function(val, [])
            elif isinstance(val, list) and val[0] == "Null":
                gvenv[gid] = NULL
            elif isinstance(val, list) and val[0] == "PrimFunc":
                prim = globals().get(unquote(val[1]))
                if prim is None:
                    raise VProcError(f"Primitive not implemented: {val[1]}")
                gvenv[gid] = prim
            else:
                gvenv[gid] = val
    return gvenv

# ── Conversion ────────────────────────────────────────────────────────────────

def parse_lvenv_node(node, gvenv):
    if not isinstance(node, list) or not node or node[0] != "__LVEnv__": return []
    return [(unquote(item[0]), sexp_to_value(item[1], gvenv)) for item in node[1:]]

def sexp_to_value(sexp, gvenv):
    if sexp is None: return None
    if isinstance(sexp, str):
        return NULL if sexp in ("null", "__NULL__") else sexp
    if not isinstance(sexp, list) or not sexp: return sexp
    tag = sexp[0]
    if tag == "__Closure__":
        head_node  = sexp[1] if len(sexp) > 1 else ["Head",["0","0"]]
        body_node  = sexp[2] if len(sexp) > 2 else None
        lvenv_node = sexp[3] if len(sexp) > 3 else ["__LVEnv__"]
        params = [unquote(c[2]) for c in (head_node[2:] if isinstance(head_node,list) else [])
                  if isinstance(c,list) and c[0]=="Variable"]
        inner_lvenv = parse_lvenv_node(lvenv_node, gvenv)
        return Closure(params, body_node, inner_lvenv)
    if tag == "Null": return NULL
    return sexp  # raw AST

def value_to_sexp(val):
    if val is None: return ["Null",["0","0"]]
    if isinstance(val, _Null): return ["Null",["0","0"]]
    if isinstance(val, str): return val
    if isinstance(val, Closure): return closure_to_sexp(val)
    if isinstance(val, SeqClosure): return seqclosure_to_sexp(val)
    if isinstance(val, CFrame): return ["__CFrame__", value_to_sexp(val.closure), ["__SInfo__"]+(val.sinfo or [])]
    if isinstance(val, list): return val
    if isinstance(val, Stream): return val.value_to_sexp()
    if isinstance(val, Number): return val.value_to_sexp()
    if isinstance(val, LList): return val.value_to_sexp()
    if callable(val): return ["PrimFunc", quoted(val.__name__)]
    return str(val)

def closure_to_sexp(c):
    head_children = [["Variable",["0","0"],quoted(p)] for p in c.params]
    return ["__Closure__",
            ["Head",["0","0"]] + head_children,
            c.body if c.body is not None else ["Body",["0","0"]],
            lvenv_to_sexp(c.lvenv)]

def seqclosure_to_sexp(sc):
    sinfo_part = [["__SInfo__"] + sc.sinfo] if sc.sinfo else []
    body = (["Body",["0","0"]] + sinfo_part +
            [["Operator",["0","0"],["Variable",["0","0"],'"__Sink__"',["L","0"]]],
             ["OList",["0","0"]] + [value_to_sexp(v) for v in sc.olist],
             ["LCont",["0","0"],["Null",["0","0"]]]])
    return ["__Closure__",
            ["Head",["0","0"],["Variable",["0","0"],'"__Sink__"']],
            body,
            lvenv_to_sexp(sc.lvenv)]

def lvenv_to_sexp(lvenv):
    return ["__LVEnv__"] + [[quoted(n), value_to_sexp(v)] for n,v in lvenv]


# ── CChain helpers (LList-based) ──────────────────────────────────────────────

def cchain_empty():
    """Return an empty CChain (NULL)."""
    return NULL

def cchain_is_empty(cc):
    return is_null_value(cc)

def cchain_push(cc, cf):
    """Prepend CFrame cf to CChain cc, returning the new CChain."""
    return LList(cf, cc)

def cchain_pop(cc):
    """Pop the top CFrame from CChain cc.
    Returns (CFrame, rest_CChain) or raises VProcError if empty."""
    if is_null_value(cc):
        return None, NULL
    return cc.head, cc.tail

def cchain_to_list(cc):
    """Convert LList-based CChain to Python list of CFrames (for serialization)."""
    result = []
    while not is_null_value(cc):
        result.append(cc.head)
        cc = cc.tail
    return result

def list_to_cchain(frames):
    """Convert Python list of CFrames to LList-based CChain."""
    cc = NULL
    for cf in reversed(frames):
        cc = LList(cf, cc)
    return cc

# ── VProc ─────────────────────────────────────────────────────────────────────

def vproc_from_sexp(sexp, gvenv):
    if sexp[0] != "__VProc__": raise VProcError(f"Expected __VProc__")
    raw = {item[0]: item[1:] for item in sexp[1:]}

    vp = {}
    vp["sinfo"] = list(raw["__SInfo__"]) if raw.get("__SInfo__") else None
    op_items = raw.get("__Operator__", [])
    vp["operator"] = sexp_to_value(op_items[0], gvenv) if op_items else None
    vp["olist"] = [sexp_to_value(x, gvenv) for x in raw.get("__OList__", [])]

    lc_items = raw.get("__LCont__", [])
    if not lc_items:
        vp["lcont"] = None
    else:
        lc = lc_items[0]
        # (Null ...) as LCont means absent (no LCont)
        if isinstance(lc, list) and lc and lc[0] == "Null":
            vp["lcont"] = None
        else:
            vp["lcont"] = sexp_to_value(lc, gvenv)

    vp["lvenv"] = parse_lvenv_node(["__LVEnv__"] + raw.get("__LVEnv__", []), gvenv)
    vp["gvenv_files"] = [unquote(x) for x in raw.get("__GVEnv__", [])]

    _frames = []
    for item in raw.get("__CChain__", []):
        closure = sexp_to_value(item[0], gvenv)
        sinfo   = item[1][1:] if len(item) > 1 and isinstance(item[1], list) else None
        _frames.append(CFrame(closure, sinfo))
    vp["cchain"] = list_to_cchain(_frames)

    vp["pdict"] = [(sexp_to_value(p[0],gvenv), sexp_to_value(p[1],gvenv))
                   for p in raw.get("__PDict__", [])]
    return vp

def vproc_to_sexp(vp):
    lc = vp["lcont"]
    lcont_part = ["__LCont__"] + ([] if lc is None else [value_to_sexp(lc)])

    cchain_items = [[value_to_sexp(cf.closure), ["__SInfo__"]+(cf.sinfo or [])]
                    for cf in cchain_to_list(vp["cchain"])]
    return ["__VProc__",
            ["__SInfo__"] + (vp["sinfo"] or []),
            ["__Operator__", value_to_sexp(vp["operator"])],
            ["__OList__"] + [value_to_sexp(v) for v in vp["olist"]],
            lcont_part,
            lvenv_to_sexp(vp["lvenv"]),
            ["__GVEnv__"] + [quoted(f) for f in vp["gvenv_files"]],
            ["__CChain__"] + cchain_items,
            ["__PDict__"] + [[value_to_sexp(k),value_to_sexp(v)] for k,v in vp["pdict"]]]

# ── Evaluator ─────────────────────────────────────────────────────────────────

def eval_expr(expr, lvenv, gvenv):
    if not isinstance(expr, list) or not expr: raise VProcError(f"Cannot eval: {expr!r}")
    kind = expr[0]
    if kind == "Variable":
        b = expr[3] if len(expr) > 3 else None
        if isinstance(b, list) and b[0] == "L":
            idx = int(b[1])
            if idx >= len(lvenv): raise VProcError(f"LVEnv[{idx}] out of range")
            return lvenv[idx][1]
        if isinstance(b, list) and b[0] == "G":
            val = gvenv.get(int(b[1]))
            if val is None: raise VProcError(f"G{b[1]} ({unquote(expr[2])!r}) Undefined")
            return val
        raise VProcError(f"Unknown binding: {b!r}")
    if kind == "String": return expr          # preserve AST node
    if kind == "Null":   return NULL          # omitted Operator → null value
    if kind == "FuncExp": return closure_from_function(expr[2], lvenv)
    raise VProcError(f"Unknown expr kind: {kind!r}")

def extract_body(body_node, lvenv, gvenv):
    sinfo = operator_val = lcont_val = None
    olist_vals = []
    for child in body_node[2:]:
        if not isinstance(child, list): continue
        k = child[0]
        if k == "SInfo":   sinfo = child[2:]
        elif k == "Operator" and len(child) > 2:
            operator_val = eval_expr(child[2], lvenv, gvenv)
        elif k == "OList":
            olist_vals = [eval_expr(op, lvenv, gvenv) for op in child[2:]]
        elif k == "LCont" and len(child) > 2:
            lc = child[2]
            if isinstance(lc, list) and lc[0] == "Null":
                lcont_val = None    # absent
            elif isinstance(lc, list) and lc[0] == "Function":
                lcont_val = closure_from_function(lc, lvenv)
            elif isinstance(lc, list) and lc[0] == "__Closure__":
                lcont_val = sexp_to_value(lc, gvenv)
    return sinfo, operator_val, olist_vals, lcont_val

# ── Step ──────────────────────────────────────────────────────────────────────

def step(vp, gvenv):
    operator = vp["operator"]
    olist    = vp["olist"]
    lcont    = vp["lcont"]

    # Null Operator
    if is_null_value(operator):
        if lcont is not None:
            vp["operator"] = lcont
            vp["lcont"]    = None
        else:
            if cchain_is_empty(vp["cchain"]): raise VProcError(format_error("Null Operator: CChain empty, no LCont", vp.get("sinfo")))
            cf, vp["cchain"] = cchain_pop(vp["cchain"])
            vp["operator"] = cf.closure
            vp["sinfo"]    = cf.sinfo
        return

    # SeqClosure
    if isinstance(operator, SeqClosure):
        if olist:
            vp["operator"] = olist[0]
            vp["olist"]    = list(operator.olist)
            vp["lvenv"]    = operator.lvenv
        else:
            if cchain_is_empty(vp["cchain"]): raise VProcError(format_error("SeqClosure: no OList and CChain empty", vp.get("sinfo")))
            cf, vp["cchain"] = cchain_pop(vp["cchain"])
            vp["olist"]    = list(operator.olist)
            vp["operator"] = cf.closure
            vp["sinfo"]    = cf.sinfo
            vp["lvenv"]    = operator.lvenv
        return

    # ── Primitive dispatch ───────────────────────────────────────────────────
    if callable(operator):
        operator(vp, gvenv)
        return

    if is_prim_func(operator):
        prim = globals().get(unquote(operator[1]))
        if prim is None:
            raise VProcError(f"Primitive not implemented: {operator!r}")
        prim(vp, gvenv)
        return

    if not isinstance(operator, Closure):
        print(f"operator: {operator}")
        raise VProcError(f"Primitive not implemented: {operator!r}")

    params = operator.params
    body_node = operator.body
    closure_lvenv = operator.lvenv

    # Head empty
    if not params:
        cur_lvenv = vp["lvenv"]
        cur_sinfo = vp["sinfo"]
        # Step 1: push LCont if present
        if lcont is not None:
            vp["cchain"] = cchain_push(vp["cchain"], CFrame(lcont, cur_sinfo))
        # Step 2: surplus OList → SeqClosure
        if olist:
            vp["cchain"] = cchain_push(vp["cchain"], CFrame(SeqClosure(list(olist), cur_sinfo, cur_lvenv), cur_sinfo))
            vp["olist"] = []
        # Step 3: load Body
        if body_node is None: raise VProcError("Closure has no Body")
        sinfo, op_val, olist_vals, lc_val = extract_body(body_node, closure_lvenv, gvenv)
        vp["sinfo"]    = sinfo
        vp["operator"] = op_val
        vp["olist"]    = olist_vals
        vp["lcont"]    = lc_val
        vp["lvenv"]    = closure_lvenv
        return

    # Head not empty, OList not empty
    if olist:
        new_params = list(params)
        new_lvenv  = list(closure_lvenv)
        rem_olist  = list(olist)
        while new_params and rem_olist:
            new_lvenv = [(new_params.pop(0), rem_olist.pop(0))] + new_lvenv
        vp["operator"] = Closure(new_params, body_node, new_lvenv)
        vp["olist"]    = rem_olist
        return

    # Head not empty, OList empty
    if lcont is not None:
        vp["olist"]    = [operator]
        vp["operator"] = lcont
        vp["lcont"]    = None
    else:
        if cchain_is_empty(vp["cchain"]): raise VProcError(format_error("Partial apply: OList empty, no LCont, CChain empty", vp.get("sinfo")))
        cf, vp["cchain"] = cchain_pop(vp["cchain"])
        vp["olist"]    = [operator]
        vp["operator"] = cf.closure
        vp["sinfo"]    = cf.sinfo

# ── Primitive Implementations ─────────────────────────────────────────────────

def _prim_return(vp, results):
    """Standard return protocol: LCont → Operator, results → OList, LCont → absent."""
    lc = vp["lcont"]
    vp["operator"] = lc if lc is not None else NULL  # may be None → handled as Null Operator next step
    vp["olist"]    = list(results)
    vp["lcont"]    = None


def prim_is_null(vp, gvenv):
    """__is_null__ Value — passes __TRUE__ or __FALSE__ to LCont."""
    if not vp["olist"]:
        raise VProcError("__is_null__: missing argument")
    val = vp["olist"].pop(0)
    # __TRUE__ = ,(t f) t,  __FALSE__ = ,(t f) f
    # Represented as Closure objects with appropriate bodies.
    result = TRUE_CLOSURE if is_null_value(val) else FALSE_CLOSURE
    _prim_return(vp, [result])


def prim_null(vp, gvenv):
    """__NULL__ — passes null to LCont."""
    _prim_return(vp, [NULL])


# ── OS Primitives ──────────────────────────────────────────────────────


def prim_OS_exit_normal(vp, gvenv):
    """__OS_exit_normal__ ,() — terminate interpreter with exit code 0."""
    sys.exit(0)


def prim_OS_exit_error(vp, gvenv):
    """__OS_exit_error__ ,() — terminate interpreter with exit code 1."""
    sys.exit(1)

# Boolean closures: ,(t f) t  and  ,(t f) f
# Head: [t, f], Body returns t (or f)
# Built as Closure objects with synthetic AST bodies.
def _make_bool_closure(param_name):
    """Return a Closure ,(t f) <param_name>."""
    head_node = ["Head", ["0","0"],
                 ["Variable", ["0","0"], '"t"'],
                 ["Variable", ["0","0"], '"f"']]
    body_node = ["Body", ["0","0"],
                 ["Operator", ["0","0"],
                  ["Null", ["0", "0"]]],
                 ["OList", ["0","0"],
                  ["Variable", ["0","0"], f'"{param_name}"',
                   ["L", "1" if param_name == "t" else "0"]]],
                 ["LCont", ["0","0"]]]
    return Closure(["t", "f"], body_node, [])

TRUE_CLOSURE  = _make_bool_closure("t")
FALSE_CLOSURE = _make_bool_closure("f")

_ESCAPE_MAP = {'"': '"', '\\': '\\', 'n': '\n', 't': '\t', 'r': '\r'}

def unescape(s):
    """Process escape sequences defined in SPEC.md: \" \\ \\n \\t \\r"""
    out = []
    it = iter(s)
    for ch in it:
        if ch == '\\':
            nxt = next(it, None)
            if nxt is None:
                raise VProcError(f"unescape: trailing backslash in {s!r}")
            replacement = _ESCAPE_MAP.get(nxt)
            if replacement is None:
                raise VProcError(f"unescape: unknown escape \\{nxt} in {s!r}")
            out.append(replacement)
        else:
            out.append(ch)
    return ''.join(out)

def _value_to_display(val):
    """Convert a VProc value to a human-readable string for output."""
    if isinstance(val, list) and val and val[0] == "String":
        # String AST node: ["String", sinfo, '"text"']
        return unescape(unquote(val[2])) if len(val) > 2 else ""
    if isinstance(val, str):
        return unescape(unquote(val))
    # Closure, SeqClosure, NULL, raw list → render as S-expression
    return sexp_to_str(value_to_sexp(val))


def prim_OS_print(vp, gvenv):
    """__OS_print__ Value — prints Value to stdout, resumes the LCont with an empty OList."""
    if not vp["olist"]:
        raise VProcError("__OS_print__: missing argument")
    val = vp["olist"].pop(0)
    print(_value_to_display(val), end='')
    _prim_return(vp, [])

def prim_OS_print_error(vp, gvenv):
    """__OS_print_error__ Value — prints Value to stderr, resumes the LCont with an empty OList."""
    if not vp["olist"]:
        raise VProcError("__OS_print_error__: missing argument")
    val = vp["olist"].pop(0)
    print(_value_to_display(val), end='', file=sys.stderr)
    _prim_return(vp, [])


def _prim_error(vp, on_error, message):
    """Invoke onError closure with an error message string."""
    vp["operator"] = on_error
    vp["olist"]    = [quoted(message)]
    # do not clear lcont to continue after error handling
    # vp["lcont"]    = None


def prim_OS_pipe(vp, gvenv):
    """__OS_pipe__ ,(inputStream outputStream) — create an OS pipe."""
    import os
    r_fd, w_fd = os.pipe()
    _prim_return(vp, [Stream(r_fd, 'r'), Stream(w_fd, 'w')])


def prim_OS_read(vp, gvenv):
    """__OS_read__ onError inputStream ,(String)
    Non-blocking read from inputStream (up to 4096 bytes).
      "..."  — data read (may be a partial line; includes trailing newline if full line)
      ""     — no data available yet (writer still open)
      null   — EOF (writer closed)
    On error, calls onError with an error message string.
    """
    import fcntl, os
    olist = vp["olist"]
    if len(olist) < 2:
        raise VProcError("__OS_read__: missing argument(s)")
    on_error = olist.pop(0)
    stream   = olist.pop(0)
    if not isinstance(stream, Stream) or stream.mode != 'r':
        _prim_error(vp, on_error, f"__OS_read__: expected readable Stream, got {stream!r}")
        return
    try:
        # set O_NONBLOCK on the raw fd
        flags = fcntl.fcntl(stream.fd, fcntl.F_GETFL)
        fcntl.fcntl(stream.fd, fcntl.F_SETFL, flags | os.O_NONBLOCK)
        data = os.read(stream.fd, 4096)
        if data == b"":
            _prim_return(vp, [NULL])                          # EOF
        else:
            _prim_return(vp, [quoted(data.decode('utf-8'))])  # data
    except BlockingIOError:
        _prim_return(vp, [quoted("")])                        # no data yet
    except OSError as e:
        _prim_error(vp, on_error, f"__OS_read__: {e}")


def prim_OS_write(vp, gvenv):
    """__OS_write__ onError outputStream String ,()
    Write String to outputStream. No newline is appended.
    On success, resumes LCont with an empty OList.
    On error, calls onError with an error message string.
    """
    import os
    olist = vp["olist"]
    if len(olist) < 3:
        raise VProcError("__OS_write__: missing argument(s)")
    on_error = olist.pop(0)
    stream   = olist.pop(0)
    val      = olist.pop(0)
    if not isinstance(stream, Stream) or stream.mode != 'w':
        _prim_error(vp, on_error, f"__OS_write__: expected writable Stream, got {stream!r}")
        return
    data = _value_to_display(val).encode('utf-8')
    try:
        os.write(stream.fd, data)
    except OSError as e:
        _prim_error(vp, on_error, f"__OS_write__: {e}")
        return
    _prim_return(vp, [])


def prim_OS_close(vp, gvenv):
    """__OS_close__ onError Stream ,()
    Close inputStream or outputStream.
    On success, resumes LCont with an empty OList.
    On error, calls onError with an error message string.
    """
    olist = vp["olist"]
    if len(olist) < 2:
        raise VProcError("__OS_close__: missing argument(s)")
    on_error = olist.pop(0)
    stream   = olist.pop(0)
    if not isinstance(stream, Stream):
        _prim_error(vp, on_error, f"__OS_close__: expected Stream, got {stream!r}")
        return
    try:
        stream.close()
    except OSError as e:
        _prim_error(vp, on_error, f"__OS_close__: {e}")
        return
    _prim_return(vp, [])


# ── Number Primitives ─────────────────────────────────────────────────────────

def _check_number(name, val):
    if not isinstance(val, Number):
        raise VProcError(f"{name}: expected Number, got {val!r}")

def prim_Number_fromString(vp, gvenv):
    """__Number_fromString__ onError String ,(Number)"""
    olist = vp["olist"]
    if len(olist) < 2:
        raise VProcError("__Number_fromString__: missing argument(s)")
    on_error = olist.pop(0)
    val      = olist.pop(0)
    s = unescape(unquote(val)) if isinstance(val, str) else _value_to_display(val)
    try:
        n = int(s)
    except ValueError:
        try:
            n = float(s)
        except ValueError:
            _prim_error(vp, on_error, f"__Number_fromString__: invalid number: {s!r}")
            return
    _prim_return(vp, [Number(n)])

def prim_Number_toString(vp, gvenv):
    """__Number_toString__ Number ,(String)"""
    olist = vp["olist"]
    if not olist: raise VProcError("__Number_toString__: missing argument")
    val = olist.pop(0)
    _check_number("__Number_toString__", val)
    n = val.value
    s = str(int(n)) if isinstance(n, float) and n == int(n) else str(n)
    _prim_return(vp, [quoted(s)])

def prim_Number_is_zero(vp, gvenv):
    """__Number_is_zero__ Number ,(Boolean)"""
    olist = vp["olist"]
    if not olist: raise VProcError("__Number_is_zero__: missing argument")
    val = olist.pop(0)
    _check_number("__Number_is_zero__", val)
    _prim_return(vp, [TRUE_CLOSURE if val.value == 0 else FALSE_CLOSURE])

def prim_Number_is_positive(vp, gvenv):
    """__Number_is_positive__ Number ,(Boolean)"""
    olist = vp["olist"]
    if not olist: raise VProcError("__Number_is_positive__: missing argument")
    val = olist.pop(0)
    _check_number("__Number_is_positive__", val)
    _prim_return(vp, [TRUE_CLOSURE if val.value > 0 else FALSE_CLOSURE])

def prim_Number_is_negative(vp, gvenv):
    """__Number_is_negative__ Number ,(Boolean)"""
    olist = vp["olist"]
    if not olist: raise VProcError("__Number_is_negative__: missing argument")
    val = olist.pop(0)
    _check_number("__Number_is_negative__", val)
    _prim_return(vp, [TRUE_CLOSURE if val.value < 0 else FALSE_CLOSURE])

def _prim_binop(vp, name, op):
    olist = vp["olist"]
    if len(olist) < 2: raise VProcError(f"{name}: missing argument(s)")
    a = olist.pop(0)
    b = olist.pop(0)
    _check_number(name, a)
    _check_number(name, b)
    _prim_return(vp, [Number(op(a.value, b.value))])

def prim_Number_add(vp, gvenv):
    """__Number_add__ Number Number ,(Number)"""
    _prim_binop(vp, "__Number_add__", lambda a, b: a + b)

def prim_Number_subtract(vp, gvenv):
    """__Number_subtract__ Number Number ,(Number)"""
    _prim_binop(vp, "__Number_subtract__", lambda a, b: a - b)

def prim_Number_multiply(vp, gvenv):
    """__Number_multiply__ Number Number ,(Number)"""
    _prim_binop(vp, "__Number_multiply__", lambda a, b: a * b)

def prim_Number_divide(vp, gvenv):
    """__Number_divide__ onError Number Number ,(Number)"""
    olist = vp["olist"]
    if len(olist) < 3: raise VProcError("__Number_divide__: missing argument(s)")
    on_error = olist.pop(0)
    a = olist.pop(0)
    b = olist.pop(0)
    _check_number("__Number_divide__", a)
    _check_number("__Number_divide__", b)
    if b.value == 0:
        _prim_error(vp, on_error, "__Number_divide__: division by zero")
        return
    result = a.value / b.value
    # preserve int when possible
    if isinstance(a.value, int) and isinstance(b.value, int) and result == int(result):
        result = int(result)
    _prim_return(vp, [Number(result)])

def prim_Number_floor(vp, gvenv):
    """__Number_floor__ Number ,(Number)"""
    import math
    olist = vp["olist"]
    if not olist: raise VProcError("__Number_floor__: missing argument")
    val = olist.pop(0)
    _check_number("__Number_floor__", val)
    _prim_return(vp, [Number(int(math.floor(val.value)))])


# ── CChain / CFrame Primitives ────────────────────────────────────────────────

def prim_CChain_get(vp, gvenv):
    """__CChain_get__ ,(CChain) — pass current CChain (LList) to LCont."""
    _prim_return(vp, [vp["cchain"]])


def prim_CChain_set(vp, gvenv):
    """__CChain_set__ CChain ,() — replace current CChain; invoke LCont directly."""
    olist = vp["olist"]
    if not olist:
        raise VProcError("__CChain_set__: missing argument")
    cc = olist.pop(0)
    if not (is_null_value(cc) or isinstance(cc, LList)):
        raise VProcError(f"__CChain_set__: expected CChain (LList or null), got {cc!r}")
    vp["cchain"] = cc
    lc = vp["lcont"]
    vp["operator"] = lc if lc is not None else NULL
    vp["olist"]    = []
    vp["lcont"]    = None


def prim_CChain_push_CFrame(vp, gvenv):
    """__CChain_push_CFrame__ CFrame — push CFrame onto CChain; invoke LCont with no arg."""
    olist = vp["olist"]
    if not olist:
        raise VProcError("__CChain_push_CFrame__: missing argument")
    cf = olist.pop(0)
    if not isinstance(cf, CFrame):
        raise VProcError(f"__CChain_push_CFrame__: expected CFrame, got {cf!r}")
    vp["cchain"] = cchain_push(vp["cchain"], cf)
    _prim_return(vp, [])


def prim_CChain_pop_CFrame(vp, gvenv):
    """__CChain_pop_CFrame__ ,(CFrame|null) — pop top CFrame; pass null if empty."""
    if cchain_is_empty(vp["cchain"]):
        _prim_return(vp, [NULL])
    else:
        cf, vp["cchain"] = cchain_pop(vp["cchain"])
        _prim_return(vp, [cf])


# ── LList Primitives ──────────────────────────────────────────────────────────

def prim_LList_cons(vp, gvenv):
    """__LList_cons__ head tail ,(newList)"""
    olist = vp["olist"]
    if len(olist) < 2:
        raise VProcError("__LList_cons__: missing argument(s)")
    head = olist.pop(0)
    tail = olist.pop(0)
    _prim_return(vp, [LList(head, tail)])


def prim_LList_uncons(vp, gvenv):
    """__LList_uncons__ onError list ,(head tail)"""
    olist = vp["olist"]
    if len(olist) < 2:
        raise VProcError("__LList_uncons__: missing argument(s)")
    on_error = olist.pop(0)
    lst      = olist.pop(0)
    if not isinstance(lst, LList):
        _prim_error(vp, on_error, f"__LList_uncons__: expected LList, got {lst!r}")
        return
    _prim_return(vp, [lst.head, lst.tail])


# ── CFrame Primitives ─────────────────────────────────────────────────────────

def prim_CFrame_get_Closure(vp, gvenv):
    """__CFrame_get_Closure__ CFrame ,(Closure) — pass CFrame's closure to LCont."""
    olist = vp["olist"]
    if not olist:
        raise VProcError("__CFrame_get_Closure__: missing argument")
    val = olist.pop(0)
    if not isinstance(val, CFrame):
        raise VProcError(f"__CFrame_get_Closure__: expected CFrame, got {val!r}")
    _prim_return(vp, [val.closure])

def prim_CFrame_get_SInfo(vp, gvenv):
    """__CFrame_get_SInfo__ CFrame ,(String)"""
    olist = vp["olist"]
    if not olist:
        raise VProcError("__CFrame_get_SInfo__: missing argument")
    val = olist.pop(0)
    if not isinstance(val, CFrame):
        raise VProcError(f"__CFrame_get_SInfo__: expected CFrame, got {val!r}")
    s = sinfo_to_str(val.sinfo) or ""
    _prim_return(vp, [quoted(s)])


PRIMITIVES = {
    "__is_null__":             prim_is_null,
    "__OS_exit_normal__":      prim_OS_exit_normal,
    "__OS_exit_error__":       prim_OS_exit_error,
    "__OS_print__":            prim_OS_print,
    "__OS_print_error__":      prim_OS_print_error,
    "__OS_pipe__":             prim_OS_pipe,
    "__OS_read__":             prim_OS_read,
    "__OS_write__":            prim_OS_write,
    "__OS_close__":            prim_OS_close,
    "__Number_fromString__":   prim_Number_fromString,
    "__Number_toString__":     prim_Number_toString,
    "__Number_is_zero__":      prim_Number_is_zero,
    "__Number_is_positive__":  prim_Number_is_positive,
    "__Number_is_negative__":  prim_Number_is_negative,
    "__Number_add__":          prim_Number_add,
    "__Number_subtract__":     prim_Number_subtract,
    "__Number_multiply__":     prim_Number_multiply,
    "__Number_divide__":       prim_Number_divide,
    "__Number_floor__":        prim_Number_floor,
    "__CChain_get__":          prim_CChain_get,
    "__CChain_set__":          prim_CChain_set,
    "__CChain_push_CFrame__":  prim_CChain_push_CFrame,
    "__CChain_pop_CFrame__":   prim_CChain_pop_CFrame,
    "__LList_cons__":          prim_LList_cons,
    "__LList_uncons__":        prim_LList_uncons,
    "__CFrame_get_Closure__":  prim_CFrame_get_Closure,
    "__CFrame_get_SInfo__":    prim_CFrame_get_SInfo,
}

# ── Main ──────────────────────────────────────────────────────────────────────

def main():
    if len(sys.argv) != 2:
        print("usage: vproc_step.py <module_file>", file=sys.stderr); sys.exit(1)
    gvenv = load_module(parse_sexp(open(sys.argv[1]).read()))
    vp    = vproc_from_sexp(parse_sexp(sys.stdin.read()), gvenv)
    try:
        step(vp, gvenv)
    except VProcError as e:
        print(f"Error: {e}", file=sys.stderr); sys.exit(1)
    print(sexp_to_str(vproc_to_sexp(vp)))

if __name__ == "__main__":
    main()
