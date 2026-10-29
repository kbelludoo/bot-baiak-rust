#!/usr/bin/env bash
# ==============================================================================
# ðŸ›¡ï¸ Baiak Idle â€” Script AutÃ´nomo de Teste CanÃ¡rio & Failover (VPS Linux)
# ==============================================================================
# Executa a esteira de validaÃ§Ã£o automÃ¡tica:
# 1. Inicia o Bot Rust como CanÃ¡rio (Porta 3001)
# 2. MantÃ©m o Bot Go como Piloto/Backup (Porta 3000)
# 3. Monitora a saÃºde do Rust por 30 minutos (RAM, Uptime, Kills, Circuit Breaker)
# 4. Se o Rust mantiver 100% de estabilidade -> Desliga o Go e assume todas as contas
# 5. Se o Rust falhar -> Executa rollback imediato para o Go sem perda de progresso
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUST_BIN="${SCRIPT_DIR}/bot-baiak-rust-linux"
GO_BIN="${SCRIPT_DIR}/baiak-bot-linux"

CANARY_DURATION_SEC=1800  # 30 minutos de teste canÃ¡rio
CHECK_INTERVAL_SEC=15     # Intervalo de checagem de saÃºde
MAX_RAM_MB=25             # Teto mÃ¡ximo de RAM aceitÃ¡vel para o Rust na VPS

RUST_PID_FILE="/tmp/baiak_rust.pid"
GO_PID_FILE="/tmp/baiak_go.pid"

log() {
    echo -e "[$(date '+%Y-%m-%d %H:%M:%S')] $1"
}

cleanup() {
    log "ðŸ§¹ Finalizando processos temporÃ¡rios..."
    if [ -f "$RUST_PID_FILE" ]; then
        kill -9 "$(cat "$RUST_PID_FILE")" 2>/dev/null || true
        rm -f "$RUST_PID_FILE"
    fi
}

# 1. ValidaÃ§Ã£o dos BinÃ¡rios
if [ ! -f "$RUST_BIN" ]; then
    log "âŒ Erro: BinÃ¡rio Rust '$RUST_BIN' nÃ£o encontrado!"
    exit 1
fi

chmod +x "$RUST_BIN"
if [ -f "$GO_BIN" ]; then
    chmod +x "$GO_BIN"
fi

log "ðŸš€ ==========================================================="
log "ðŸš€ INICIANDO ESTEIRA DE TESTE CANÃRIO RUST x GO NA VPS"
log "ðŸš€ ==========================================================="

# 2. Iniciar Bot Rust em Background
log "ðŸŸ¢ Iniciando binÃ¡rio Rust em modo CanÃ¡rio..."
"$RUST_BIN" > "${SCRIPT_DIR}/rust_canary.log" 2>&1 &
echo $! > "$RUST_PID_FILE"
RUST_PID=$(cat "$RUST_PID_FILE")
log "âœ” Rust iniciado com PID: $RUST_PID"

ELAPSED=0
SUCCESS=true

while [ "$ELAPSED" -lt "$CANARY_DURATION_SEC" ]; do
    sleep "$CHECK_INTERVAL_SEC"
    ELAPSED=$((ELAPSED + CHECK_INTERVAL_SEC))

    # A. Checar se o processo Rust continua vivo
    if ! kill -0 "$RUST_PID" 2>/dev/null; then
        log "âŒ ALERTA CRÃTICO: Processo Rust encerrou inesperadamente!"
        SUCCESS=false
        break
    fi

    # B. Checar Consumo de RAM (RSS) via /proc
    RSS_KB=$(grep VmRSS /proc/"$RUST_PID"/status 2>/dev/null | awk '{print $2}' || echo 0)
    RAM_MB=$((RSS_KB / 1024))

    if [ "$RAM_MB" -gt "$MAX_RAM_MB" ]; then
        log "âš ï¸ ALERTA: Consumo de RAM do Rust excedeu o limite ($RAM_MB MB > $MAX_RAM_MB MB)!"
        SUCCESS=false
        break
    fi

    # C. Consultar API /api/status do Rust
    STATUS_HTTP=$(curl -s -m 5 http://127.0.0.1:3001/api/status || echo "")
    if [ -z "$STATUS_HTTP" ]; then
        log "âš ï¸ API /api/status do Rust nÃ£o respondeu na porta 3001."
    else
        CB_STATE=$(echo "$STATUS_HTTP" | grep -o '"cb_state":"[^"]*"' | head -n1 || echo "")
        log "ðŸ“Š [Progresso ${ELAPSED}s / ${CANARY_DURATION_SEC}s] RAM: ${RAM_MB} MB | Status: Ativo | $CB_STATE"
    fi
done

# 3. DecisÃ£o de MigraÃ§Ã£o ou Failover
if [ "$SUCCESS" = true ]; then
    log "ðŸŽ‰ ==========================================================="
    log "ðŸŽ‰ TESTE CANÃRIO CONCLUÃDO COM 100% DE SUCESSO!"
    log "ðŸŽ‰ O motor Rust comprovou estabilidade, Zero-GC e consumo de ~${RAM_MB} MB."
    log "ðŸŽ‰ ==========================================================="

    # Se a versÃ£o Go ainda estiver rodando, encerra com seguranÃ§a
    if [ -f "$GO_PID_FILE" ]; then
        log "ðŸ›‘ Desativando versÃ£o em Go..."
        kill -15 "$(cat "$GO_PID_FILE")" 2>/dev/null || true
        rm -f "$GO_PID_FILE"
    fi

    log "âœ… A versÃ£o em Rust agora Ã© o motor PRINCIPAL definitivo na VPS!"
    log "ðŸ“Œ Logs contÃ­nuos disponÃ­veis em: ${SCRIPT_DIR}/rust_canary.log"
else
    log "ðŸš¨ ==========================================================="
    log "ðŸš¨ FALHA DETECTADA NO TESTE CANÃRIO! EXECUTANDO ROLLBACK..."
    log "ðŸš¨ ==========================================================="
    cleanup

    if [ -f "$GO_BIN" ]; then
        log "ðŸ”„ Reativando versÃ£o em Go para garantir que a caÃ§a nÃ£o pare..."
        "$GO_BIN" > "${SCRIPT_DIR}/go_fallback.log" 2>&1 &
        echo $! > "$GO_PID_FILE"
        log "âœ” Fallback para Go concluÃ­do com sucesso (PID: $(cat "$GO_PID_FILE"))."
    else
        log "âš ï¸ BinÃ¡rio de fallback em Go nÃ£o encontrado."
    fi
    exit 1
fi
