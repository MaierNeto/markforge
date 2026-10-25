#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""check_sem_ci_github — trava anti-retorno da RFC-013 (GitHub é só armazenamento).

Varre os arquivos de `.github/` do repositório — rastreados pelo git MAIS os não
rastreados não ignorados (`git ls-files .github` e `git ls-files --others
--exclude-standard .github`) — e REPROVA qualquer automação:

  workflow-actions  .github/workflows/*.yml|yaml — GitHub Actions aposentado
  dependabot        .github/dependabot.yml|yaml  — Dependabot é automação
  config-bot        .github/auto-assign|lock|stale.yml|yaml — config de bot
  sync-externo      .github/<...sync...>.yml|yaml — sincronização externa

Fica (não reprova): conteúdo humano de `.github/` — ISSUE_TEMPLATE/,
PULL_REQUEST_TEMPLATE.md, CODEOWNERS, docs. O instrumento imprime caminho e
regra, nunca conteúdo. O zero também é impresso: sem `.github/` o laudo declara
quantos arquivos conferiu (o agregado escreve o zero do estado que não ocorreu).

Duas varreduras, porque o GitHub Actions lê o workflow da ARVORE DO REF
empurrado (não do main): (1) o tree de trabalho atual; (2) a árvore de cada
ponta de ref em `refs/heads` e `refs/remotes` — um workflow aposentado só no
`main` continua vivo e empurrawel num ponteiro velho (incidente 25/10:
`fix-painel2` re-disparou o `piso.yml` já aposentado).

Modos:
  python scripts/check_sem_ci_github.py [RAIZ]   # RAIZ default: cwd
  python scripts/check_sem_ci_github.py --selftest

Stdlib puro, sem rede (NFR-001). Saída ASCII (console cp1252 da estação).
Exit 0 = limpo · 1 = achado · 2 = erro de uso ou git/consulta indisponível
(consulta que falha nunca é verde).
"""
import os
import shutil
import subprocess
import sys
import tempfile

if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")

EXT = (".yml", ".yaml")

# (regra, descrição, predicado sobre o caminho relativo com barras)
REGRAS = [
    ("workflow-actions",
     "GitHub Actions esta aposentado (RFC-013): o GitHub e so armazenamento",
     lambda rel: rel.startswith(".github/workflows/") and rel.endswith(EXT)),
    ("dependabot",
     "config do Dependabot e automacao; sai do repo (RFC-013)",
     lambda rel: rel in (".github/dependabot.yml", ".github/dependabot.yaml")),
    ("config-bot",
     "config de bot (auto-assign/lock/stale) e automacao; sai do repo (RFC-013)",
     lambda rel: rel.startswith(".github/") and
                  os.path.basename(rel) in ("auto-assign.yml", "auto-assign.yaml",
                                            "lock.yml", "lock.yaml",
                                            "stale.yml", "stale.yaml")),
    ("sync-externo",
     "config de sincronizacao externa (repo-sync, config-sync) e automacao (RFC-013)",
     lambda rel: rel.startswith(".github/") and rel.endswith(EXT) and
                  "sync" in os.path.splitext(os.path.basename(rel))[0]),
]

BOT_BASENAMES = ("auto-assign.yml", "auto-assign.yaml",
                 "lock.yml", "lock.yaml", "stale.yml", "stale.yaml")


def classificar(rel):
    """Primeira regra que casa decide; caminho fora de .github/ nao e automacao."""
    for regra, desc, casa in REGRAS:
        if casa(rel):
            return regra, desc
    return None


def _git_ls(args, raiz):
    try:
        p = subprocess.run(["git", "-C", raiz] + args, capture_output=True)
    except FileNotFoundError:
        return None, "git ausente nesta estacao — o varredor precisa de git para listar .github/"
    if p.returncode != 0:
        motivo = p.stderr.decode("utf-8", "replace").strip()[:160]
        return None, "git ls-files falhou em %s: %s" % (raiz, motivo)
    return [x for x in p.stdout.decode("utf-8", "replace").split("\0") if x], None


def listar_github(raiz):
    """Rastreados + nao rastreados nao ignorados sob .github/ (iscas novas do tree contam)."""
    tracked, err = _git_ls(["ls-files", "-z", "--", ".github"], raiz)
    if err:
        return None, err
    others, err = _git_ls(["ls-files", "-z", "--others", "--exclude-standard", "--", ".github"], raiz)
    if err:
        return None, err
    return sorted(set(tracked) | set(others)), None


def varrer(raiz):
    """Retorna (achados, conferidos, erro). Achado = (caminho, regra, descricao); nunca conteudo."""
    arquivos, err = listar_github(raiz)
    if err:
        return [], 0, err
    achados = []
    for rel in arquivos:
        veredito = classificar(rel)
        if veredito:
            achados.append((rel, veredito[0], veredito[1]))
    return achados, len(arquivos), None


# Um workflow aposentado SO no main nao basta: o GitHub Actions le o
# .github/workflows/ da ARVORE do ref empurrado, nao do main. Ponteiro velho
# (ancestral do commit de aposentadoria) ainda dispara automacao morta. Por
# isso a varredura de refs confere cada ponta empurrawel, e nao so o tree atual.
REF_NS = ("refs/heads", "refs/remotes")


def _git_stdout(args, raiz):
    try:
        p = subprocess.run(["git", "-C", raiz] + args, capture_output=True)
    except FileNotFoundError:
        return None, "git ausente nesta estacao — o varredor de refs precisa de git"
    if p.returncode != 0:
        motivo = p.stderr.decode("utf-8", "replace").strip()[:160]
        return None, "git %s falhou em %s: %s" % (" ".join(args[:2]), raiz, motivo)
    return p.stdout.decode("utf-8", "replace").splitlines(), None


def varrer_refs(raiz):
    """Confere a arvore de cada ponta de ref (heads + remotes), nao so o tree atual.

    Retorna (achados, conferidos, erro). Achado = (ref, caminho, regra, descricao).
    Ref sem commit (p.ex. repo recem-iniciado sem HEAD) e pulado — nao e achado,
    mas o zero de refs conferidos e impresso pelo chamador (nunca silencio).
    """
    refs, err = _git_stdout(["for-each-ref", "--format=%(refname)"] + list(REF_NS), raiz)
    if err:
        return [], 0, err
    achados = []
    conferidos = 0
    for ref in (r.strip() for r in refs if r.strip()):
        paths, err = _git_stdout(["ls-tree", "-r", "--name-only", ref, "--", ".github"], raiz)
        if err:
            return [], conferidos, err
        conferidos += 1
        for rel in (p.strip() for p in paths if p.strip()):
            veredito = classificar(rel)
            if veredito:
                achados.append((ref, rel, veredito[0], veredito[1]))
    return achados, conferidos, None


def cmd_varredura(raiz):
    achados, n, err = varrer(raiz)
    if err:
        print("SEM-CI-GITHUB: %s" % err)
        return 2
    achados_ref, nrefs, err = varrer_refs(raiz)
    if err:
        print("SEM-CI-GITHUB: %s" % err)
        return 2
    for rel, regra, desc in achados:
        print("ACHADO: %s :: %s — %s" % (rel, regra, desc))
    for ref, rel, regra, desc in achados_ref:
        print("ACHADO-REF: %s :: %s :: %s — %s" % (ref, rel, regra, desc))
    print("SEM-CI-GITHUB: verificados %d arquivo(s) em .github/ no tree; %d achado(s)" % (n, len(achados)))
    print("SEM-CI-GITHUB: conferidas %d ponta(s) de ref; %d achado(s) em arvores de ref" % (nrefs, len(achados_ref)))
    return 1 if (achados or achados_ref) else 0


# ---------------------------------------------------------------- selftest

def _escreve(raiz, rel, texto="isca-contra-exemplo RFC-013\n"):
    destino = os.path.join(raiz, *rel.split("/"))
    os.makedirs(os.path.dirname(destino), exist_ok=True)
    with open(destino, "w", encoding="utf-8", newline="\n") as f:
        f.write(texto)


def _git(raiz, *args):
    p = subprocess.run(["git", "-C", raiz] + list(args), capture_output=True)
    if p.returncode != 0:
        raise RuntimeError("git %s falhou: %s" % (" ".join(args),
                                                  p.stderr.decode("utf-8", "replace").strip()))


def _fixture(diretorio, tracked_rel, untracked_rel):
    _git(diretorio, "init", "-q")
    for rel in tracked_rel:
        _escreve(diretorio, rel)
    for rel in untracked_rel:
        _escreve(diretorio, rel)
    if tracked_rel:
        _git(diretorio, "add", "--", ".github")


ISCAS = {
    ".github/workflows/ci.yml": "workflow-actions",
    ".github/workflows/sub/reuse.yaml": "workflow-actions",
    ".github/dependabot.yml": "dependabot",
    ".github/auto-assign.yml": "config-bot",
    ".github/lock.yaml": "config-bot",
    ".github/stale.yml": "config-bot",
    ".github/sync.yml": "sync-externo",
    ".github/repo-sync.yaml": "sync-externo",
}
# isca nao rastreada: criada no tree, ainda sem git add — o portao local roda
# ANTES do commit, entao automacao nova tem de morder mesmo assim.
ISCA_UNTRACKED = ".github/workflows/isca-suja.yml"
CONTRA_EXEMPLOS = [
    ".github/CODEOWNERS",
    ".github/ISSUE_TEMPLATE/bug_report.md",
    ".github/PULL_REQUEST_TEMPLATE.md",
    ".github/workflows/LEIA-ME.md",  # humano ate dentro de workflows/ passa
    "docs/qualquer-coisa.yml",       # yml fora de .github/ nao e automacao
]


def selftest():
    casos = 0
    falhas = []
    base = tempfile.mkdtemp(prefix="sem-ci-github-selftest-")
    try:
        # caso 1 — isca E contra-exemplo no MESMO fixture; morder quem deve,
        # nao morder quem nao deve; rc de achado.
        d1 = os.path.join(base, "misturado")
        os.makedirs(d1)
        _fixture(d1, list(ISCAS) + CONTRA_EXEMPLOS, [ISCA_UNTRACKED])
        achados, n, err = varrer(d1)
        casos += 1
        if err:
            falhas.append("misturado: consulta falhou (%s)" % err)
        else:
            got = {rel: regra for rel, regra, _ in achados}
            esperado = dict(ISCAS)
            esperado[ISCA_UNTRACKED] = "workflow-actions"
            if got != esperado:
                sobra = sorted(set(got) - set(esperado))
                falta = sorted(set(esperado) - set(got))
                falhas.append("misturado: sobra=%r falta=%r" % (sobra, falta))
            humanos_mordidos = sorted(set(got) & set(CONTRA_EXEMPLOS))
            if humanos_mordidos:
                falhas.append("misturado: conteudo humano mordido %r" % humanos_mordidos)
            n_esperado = (len(ISCAS) + 1 +
                          sum(1 for c in CONTRA_EXEMPLOS if c.startswith(".github/")))
            if n != n_esperado:
                falhas.append("misturado: contagem de conferidos=%d esperado=%d"
                              % (n, n_esperado))
            casos += 1
            if cmd_varredura(d1) != 1:
                falhas.append("misturado: rc de achado nao foi 1")

        # caso 2 — cobertura de regra: cada regra casa ao menos uma isca.
        cobertas = {regra for _, regra, _ in achados}
        casos += 1
        for regra, _, _ in REGRAS:
            if regra not in cobertas:
                falhas.append("cobertura: regra %s nao foi mordida por nenhuma isca" % regra)

        # caso 3 — contra-exemplo puro: so conteudo humano passa verde.
        d2 = os.path.join(base, "humano")
        os.makedirs(d2)
        _fixture(d2, CONTRA_EXEMPLOS, [])
        ach2, n2, err2 = varrer(d2)
        casos += 1
        if err2 or ach2:
            falhas.append("humano: deveria passar limpo, deu %r (%s)" % (ach2, err2))
        casos += 1
        if cmd_varredura(d2) != 0:
            falhas.append("humano: rc de limpo nao foi 0")

        # caso 4 — repo sem .github: zero impresso, exit 0.
        d3 = os.path.join(base, "vazio")
        os.makedirs(d3)
        _git(d3, "init", "-q")
        _escreve(d3, "README.md", "repo sem automacao nenhuma\n")
        _git(d3, "add", "--", "README.md")
        ach3, n3, err3 = varrer(d3)
        casos += 1
        if err3 or ach3 or n3 != 0:
            falhas.append("vazio: esperado zero arquivos em .github/, got n=%d err=%s" % (n3, err3))

        # caso 5 — cegueira morde: diretorio sem git nunca e verde.
        d4 = os.path.join(base, "sem-git")
        os.makedirs(d4)
        casos += 1
        ach4, n4, err4 = varrer(d4)
        if err4 is None:
            falhas.append("sem-git: consulta sem repositorio passou como verde")
        casos += 1
        if cmd_varredura(d4) != 2:
            falhas.append("sem-git: rc de consulta cega nao foi 2")

        # caso 6 — o ponto cego que deixou o piso.yml voltar: automacao aposentada
        # no tree do main segue viva num REF velho, e o GitHub roda workflow da
        # arvore do ref empurrado. varrer (tree) nao ve; varrer_refs tem de morder.
        d5 = os.path.join(base, "ref-velho")
        os.makedirs(d5)
        _git(d5, "init", "-q", "-b", "main")
        _escreve(d5, ".github/workflows/piso.yml")
        _escreve(d5, "README.md", "repo nascido com automacao\n")
        _git(d5, "add", "--", ".github", "README.md")
        _git(d5, "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-q", "-m", "nascido com piso.yml")
        _git(d5, "branch", "ramo-velho")  # ponteiro parado no commit com workflow
        _git(d5, "rm", "-q", ".github/workflows/piso.yml")
        _git(d5, "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-q", "-m", "RFC-013 aposenta piso.yml no main")
        casos += 1
        ach5, n5, err5 = varrer(d5)
        if err5 or ach5:
            falhas.append("ref-velho: tree do main deveria estar limpo, deu %r (%s)" % (ach5, err5))
        ar5, nr5, err5b = varrer_refs(d5)
        casos += 1
        if err5b:
            falhas.append("ref-velho: varredura de refs falhou (%s)" % err5b)
        elif not any(a[0] == "refs/heads/ramo-velho" and a[1] == ".github/workflows/piso.yml" for a in ar5):
            falhas.append("ref-velho: ramo-velho com piso.yml nao foi mordido; got=%r" % (ar5,))
        casos += 1
        if any(a[0] == "refs/heads/main" for a in ar5):
            falhas.append("ref-velho: main limpo foi mordido indevidamente (contra-exemplo)")
        casos += 1
        if cmd_varredura(d5) != 1:
            falhas.append("ref-velho: rc de achado-em-ref-arvo nao foi 1")
    finally:
        shutil.rmtree(base, ignore_errors=True)

    for f in falhas:
        print("FALHA selftest: %s" % f)
    if falhas:
        print("SELFTEST check_sem_ci_github: FALHOU (%d caso(s), %d falha(s))" % (casos, len(falhas)))
        return 1
    print("SELFTEST check_sem_ci_github: PASSOU (%d casos; iscas do tree e de ref mordem, "
          "humanos passam, zero e impresso, cegueira nunca e verde)" % casos)
    return 0


def main(argv):
    args = [a for a in argv[1:]]
    if "--selftest" in args:
        return selftest()
    if "--help" in args or "-h" in args:
        print(__doc__)
        return 0
    raiz = "."
    restantes = [a for a in args if not a.startswith("-")]
    if len(restantes) > 1:
        print("uso: check_sem_ci_github.py [RAIZ] [--selftest]")
        return 2
    if restantes:
        raiz = restantes[0]
    return cmd_varredura(raiz)


if __name__ == "__main__":
    sys.exit(main(sys.argv))
