# Makefile — markForge (A-SDLC)
# Vocabulário de comandos padronizado

.PHONY: higiene smoke monitor test build help

help:
	@echo "Comandos disponíveis:"
	@echo "  make higiene  - Verifica se staging/disco estão limpos (sem segredos/PII)"
	@echo "  make smoke    - Verifica build e testes (tsc + vite build + vitest + cargo check)"
	@echo "  make monitor  - Mostra estado do projeto"
	@echo "  make test     - Roda testes (npm test)"
	@echo "  make build    - Build completo (npm run build)"

higiene:
	bash scripts/higiene.sh

smoke:
	bash scripts/smoke.sh

monitor:
	bash scripts/monitor.sh

test:
	npm test

build:
	npm run build
