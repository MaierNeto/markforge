#!/usr/bin/env python3
"""secret_sweep — cachorro de guarda de credenciais (diretiva do titular 13/10/2026).

Varre arquivos rastreados no git (ou o staged) procurando padrões de segredo.
Regra: credencial só em .env gitignored / env de deploy, nunca hardcoded.
Arquivo .env rastreado só vale como template (.example/.sample/.template) ou
se estiver no allowlist nominal (scripts/secret_sweep.allowlist, um path por linha).

Modos:
  python scripts/secret_sweep.py             # todos os arquivos rastreados
  python scripts/secret_sweep.py --staged    # apenas o que está staged (pre-commit)
  python scripts/secret_sweep.py --selftest  # prova de que as presas mordem

Saída: só caminho, linha, regra e FORMA do valor — jamais o próprio valor.
Exit 0 = limpo; exit 1 = achado; exit 2 = erro de uso/git.
"""
import re
import subprocess
import sys

if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")

SIGNATURES = [
    ("private-key-block", re.compile(r"-----BEGIN [A-Z ]*PRIVATE KEY-----")),
    ("aws-access-key-id", re.compile(r"\b(?:AKIA|ASIA|ABIA|ACCA)[0-9A-Z]{16}\b")),
    ("openai-anthropic-key", re.compile(r"\bsk-[A-Za-z0-9_\-]{20,}\b")),
    ("github-token", re.compile(r"\bgh[pousr]_[A-Za-z0-9]{36,}\b")),
    ("gitlab-token", re.compile(r"\bglpat-[A-Za-z0-9_\-]{20,}\b")),
    ("google-api-key", re.compile(r"\bAIza[0-9A-Za-z_\-]{35}\b")),
    ("sendgrid-key", re.compile(r"\bsg-[A-Za-z0-9_]{20,}\b")),
    ("slack-token", re.compile(r"\bxox[bpors]-[A-Za-z0-9\-]{10,}\b")),
    ("stripe-key", re.compile(r"\b[sr]k_(live|test)_[A-Za-z0-9]{20,}\b")),
    ("jwt-compact", re.compile(r"\beyJ[A-Za-z0-9_\-]{10,}\.eyJ[A-Za-z0-9_\-]{10,}\.[A-Za-z0-9_\-]{10,}\b")),
]

# atribuição genérica: KEY = valor | "KEY": "valor" | KEY: "valor"
# valor SEM aspas só vale depois de `=` (evita "secretsmanager:CreateSecret" em IAM/JSON)
GENERIC_RX = re.compile(
    r"(?i)\b([a-z0-9_]*(?:secret|password|passwd|token|api[-_]?key|access[-_]?key"
    r"|private[-_]?key|auth[-_]?token|client[-_]?secret|encryption[-_]?key)[a-z0-9_]*)\b"
    r"\s*([:=])\s*"
    r"(?:[\"']([^\"' ]{8,}(?: [^\"' ]{2,})*)[\"']|(?<==)\s*([^\s\"'`\\#,;)]{8,}))"
)

# dummy/placeholder conhecido — não morde
DUMMY = re.compile(
    r"(?i)^(?:<[^<>]*>|\{\{[^}]*\}\}|.*\b(?:dummy|demo|example|sample|changeme|replace"
    r"|placeholder|your[_-]?|seu[_-]?|sua[_-]?|testing|ci[-_](?:fake|insecure)|x{3,}|todo"
    r"|coloque|insira|preencha)\b.*)$")

# referência (variável, caminho, template Jinja), não literal — não morde
REFERENCE = re.compile(r"^(?:\$|~|/|\./|`|\[|\{[%{]|os\.|self\.[A-Za-z_]+ =|[A-Za-z0-9_]+\s+is\s+|%(\w|{))")

# contexto de código/prosa (não é literal de credencial):
#  - cadeia de atributo: self.x.y, modulo.CONST, Tipo<...>
#  - chamada/fragmento: valor termina em ( [ . OU próximo char é ( [ .
#  - palavra sem dígito com <16 chars: "nascimento", "senha-antiga", identificadores
#  - uuid canônico em arquivo de teste: fixture fake padrão
ATTR_CHAIN = re.compile(r"^[A-Za-z_]\w*\.[\w(<\[]")
UUID_CANON = re.compile(r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$")
TESTISH = re.compile(r"(?i)(^|/|_)tests?([^/]*\.py|[^/]*\.ts|[^/]*\.tsx|[^/]*\.js)|\.spec\.|\bconftest\b")


def code_prose(val: str, rest: str, path: str) -> bool:
    if "${" in val or "{{" in val:  # interpolação/template, não literal
        return True
    if ATTR_CHAIN.match(val) or (val and val[-1] in "([."):
        return True
    if rest in ("(", "[", ".", ")"):
        return True
    if not re.search(r"\d", val) and re.fullmatch(r"\w[\w\-\. ]*", val, re.UNICODE):
        return True  # sintético: só letras/hífens, sem dígito (semáforo de fixture/prosa)
    if UUID_CANON.match(val) and TESTISH.search(path):
        return True
    return False


def shape(value: str) -> str:
    m = re.sub(r"[A-Za-z]", "a", value)
    m = re.sub(r"[0-9]", "0", m)
    return m[:18]


def exempt(value: str) -> bool:
    v = value.strip()
    if len(v.split()) >= 2:  # frase/prosa, não literal
        return True
    if re.search(r"[(\[{,;]$", v):  # fragmento de código (ex.: os.environ[ )
        return True
    return bool(DUMMY.match(v) or REFERENCE.match(v))


def git(args):
    r = subprocess.run(["git"] + args, capture_output=True)
    if r.returncode != 0:
        sys.stderr.write("git %s: %s\n" % (" ".join(args), r.stderr.decode("utf-8", "replace")))
        sys.exit(2)
    return r.stdout.decode("utf-8", "replace")


def list_targets(staged: bool):
    if staged:
        out = git(["diff", "--cached", "--name-only", "--diff-filter=ACM", "-z"])
    else:
        out = git(["ls-files", "-z"])
    return [p for p in out.split("\0") if p]


def read_file(path: str, staged: bool) -> bytes:
    if staged:
        r = subprocess.run(["git", "show", ":%s" % path], capture_output=True)
        return b"" if r.returncode != 0 else r.stdout
    try:
        with open(path, "rb") as f:
            return f.read()
    except OSError:
        return b""


ENV_TRACKED_OK = re.compile(r"\.(example|sample|template)$")
ENVISH = re.compile(r"(^|/)\.env(\.|$)|\.env$|\benv\.(production|dev|homolog)\b", re.I)


def load_allowlist():
    try:
        with open("scripts/secret_sweep.allowlist", encoding="utf-8") as f:
            return {ln.strip() for ln in f if ln.strip() and not ln.strip().startswith("#")}
    except OSError:
        return set()


def scan(path: str, data: bytes, allowlist):
    findings = []
    if path in allowlist:
        return findings  # exceção nominal (canário/exemplo didático) — pular tudo
    if ENVISH.search(path) and not ENV_TRACKED_OK.search(path):
        findings.append((0, "tracked-env-file", path))
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError:
        return findings
    is_template = bool(ENV_TRACKED_OK.search(path))
    for i, line in enumerate(text.splitlines(), 1):
        commented = line.lstrip().startswith("#")
        for name, rx in SIGNATURES:
            m = rx.search(line)
            if m:
                findings.append((i, name, shape(m.group(0))))
        if commented or is_template:
            continue  # em comentário/template só as assinaturas fortes mordem
        gm = GENERIC_RX.search(line)
        if gm:
            key = gm.group(1)
            quoted = gm.group(3) is not None
            val = gm.group(3) or gm.group(4) or ""
            rest = "" if quoted else line[gm.end():gm.end() + 1]
            if not exempt(val) and not code_prose(val, rest, path):
                findings.append((i, "generic-assignment[%s]" % key, shape(val)))
    return findings


# iscas montadas em runtime: o próprio arquivo do tool nunca contém literal
# que pareça segredo (senão gates de outros repos mordem a coleira do cachorro)
_B_AWSKEY = "AKIA" + "0123456789ABCDEF"
_B_PASS = "Tr0ub4dor" + "&3xq9"
_B_TOKEN = "abc0123" + "def456"
_B_PRIVKEY = "-----BEGIN OPENSSH " + "PRIVATE KEY-----"
_B_REALTEST = "notuuid0" + "-1234-5678-9abc-def01234567890"
_B_FAKEUUID = "d34dbeef" + "-1234-5678-9abc-def012345678"
BAIT = [
    ("config.py", 'API_KEY = "%s"\n' % _B_AWSKEY, "aws-access-key-id"),
    ("settings.py", 'password = "%s"\n' % _B_PASS, "generic-assignment"),
    ("deploy.sh", "export ASAAS_TOKEN=%s\n" % _B_TOKEN, "generic-assignment"),
    ("id_ed25519", _B_PRIVKEY + "\nabc\n", "private-key-block"),
    ("prod.env", "SECRET_KEY=abc\n", "tracked-env-file"),
    ("ok.py", 'password = "ci-fake-not-a-secret"\n', None),
    (".env.sample", "PASSWORD=anything-here\n", None),
    ("policy.json", '"Action": "secretsmanager:CreateSecret"\n', None),
    ("notas.md", "TOKEN: **o valor vive em ~/.coolify_token** no servidor\n", None),
    ("sshd.md", 'PasswordAuthentication: "without-password"\n', None),
    ("ref.py", 'api_key = os.environ["API_KEY"]\n', None),
    ("chain.py", 'password = self.user.password_reset\n', None),
    ("call.py", 'token = generate_token()\n', None),
    ("tests.py", 'WEBHOOK_TOKEN = "%s"\n' % _B_FAKEUUID, None),
    ("fixtures.py", 'WEBHOOK_TOKEN = "fake-not-a-real-webhook-value"\n', None),
    ("real_test.py", 'WEBHOOK_TOKEN = "%s"\n' % _B_REALTEST, "generic-assignment"),
    ("prosa.md", 'a senha inicial vira password: nascimento\n', None),
]


def selftest():
    import os
    import tempfile
    cwd = os.getcwd()
    failures = 0
    with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as td:
        os.chdir(td)
        subprocess.run(["git", "init", "-q", "."], check=True)
        for name, content, expect in BAIT:
            p = os.path.join(td, name)
            with open(p, "w", encoding="utf-8") as f:
                f.write(content)
            subprocess.run(["git", "add", "-f", "--", name], check=True)
            got = scan(name, content.encode("utf-8"), set())
            if expect is None:
                if got:
                    print("FALHA selftest: %s deveria passar limpo, achou %r" % (name, got))
                    failures += 1
                else:
                    print("ok (limpo esperado): %s" % name)
            else:
                if any(expect in hit[1] for hit in got):
                    print("ok (mordeu): %s -> %s" % (name, expect))
                else:
                    print("FALHA selftest: %s nao mordido por %s (got %r)" % (name, expect, got))
                    failures += 1
        os.chdir(cwd)
    print("SELFTEST: %s" % ("PASSOU" if failures == 0 else "%d FALHA(S)" % failures))
    return 0 if failures == 0 else 1


def main():
    staged = "--staged" in sys.argv
    if "--selftest" in sys.argv:
        return selftest()
    allowlist = load_allowlist()
    total = 0
    for path in list_targets(staged):
        data = read_file(path, staged)
        if not data or len(data) > 2_000_000:
            continue
        for lineno, rule, form in scan(path, data, allowlist):
            if lineno:
                print("%s:%s %s valor-forma=%s" % (path, lineno, rule, form))
            else:
                print("%s %s" % (path, rule))
            total += 1
    if total:
        print("SECRET_SWEEP: %d achado(s) — credencial só em .env gitignored/env de deploy." % total)
        return 1
    print("SECRET_SWEEP: limpo (%s)" % ("staged" if staged else "rastreados"))
    return 0


if __name__ == "__main__":
    sys.exit(main())
