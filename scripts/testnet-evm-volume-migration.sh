#!/usr/bin/env bash
set -Eeuo pipefail

# Destructive testnet runbook:
#   1. Scale payy-evm to 0.
#   2. Scale Blockscout workloads to 0.
#   3. Mount the payy-evm PVC into a one-shot pod and run in-place migration.
#   4. Delete the migration pod.
#   5. Reset Blockscout PostgreSQL and Redis.
#   6. Update payy-evm to the resolved pinned image, scale it to 1, then restore Blockscout
#      replicas.
#
# Required:
#   CONFIRM=run-testnet-evm-migration
#
# Preview without mutating Kubernetes:
#   DRY_RUN=true scripts/testnet-evm-volume-migration.sh
#
# Defaults match helm/payy-evm-testnet-values.yaml and helm/blockscout/testnet-values.yaml.

KUBECTL="${KUBECTL:-kubectl}"
NAMESPACE="${NAMESPACE:-testnet-evm}"
PAYY_EVM_STATEFULSET="${PAYY_EVM_STATEFULSET:-payy-evm}"
PAYY_EVM_CONTAINER="${PAYY_EVM_CONTAINER:-payy-evm}"
PAYY_EVM_IMAGE_SOURCE="${PAYY_EVM_IMAGE:-gcr.io/polybase-testnet/payy-evm-testnet:latest}"
PAYY_EVM_PIN_IMAGE="${PAYY_EVM_PIN_IMAGE:-true}"
PAYY_EVM_IMAGE="${PAYY_EVM_IMAGE_SOURCE}"
PAYY_EVM_IMAGE_DIGEST=""
PAYY_EVM_PVC="${PAYY_EVM_PVC:-data-payy-evm-0}"
PAYY_EVM_DATADIR="${PAYY_EVM_DATADIR:-/data/sequencer}"
MIGRATION_POD="${MIGRATION_POD:-payy-evm-volume-migration}"
MIGRATION_TIMEOUT="${MIGRATION_TIMEOUT:-12h}"
PAYY_EVM_READY_TIMEOUT="${PAYY_EVM_READY_TIMEOUT:-15m}"
PAYY_EVM_PATCH_RUN_SUBCOMMAND="${PAYY_EVM_PATCH_RUN_SUBCOMMAND:-true}"

BLOCKSCOUT_RESOURCE_REGEX="${BLOCKSCOUT_RESOURCE_REGEX:-blockscout}"
BLOCKSCOUT_SELECTOR="${BLOCKSCOUT_SELECTOR:-app.kubernetes.io/instance=blockscout}"
BLOCKSCOUT_DB_RESET_MODE="${BLOCKSCOUT_DB_RESET_MODE:-fresh-cloud-sql}"
BLOCKSCOUT_DB_SECRET="${BLOCKSCOUT_DB_SECRET:-blockscout-database-secret}"
BLOCKSCOUT_DB_SECRET_KEY="${BLOCKSCOUT_DB_SECRET_KEY:-DATABASE_URL}"
BLOCKSCOUT_DB_RESET_POD="${BLOCKSCOUT_DB_RESET_POD:-blockscout-db-reset}"
BLOCKSCOUT_DB_RESET_IMAGE="${BLOCKSCOUT_DB_RESET_IMAGE:-postgres:16}"
BLOCKSCOUT_CLOUDSQL_PROJECT="${BLOCKSCOUT_CLOUDSQL_PROJECT:-polybase-testnet}"
BLOCKSCOUT_CLOUDSQL_INSTANCE="${BLOCKSCOUT_CLOUDSQL_INSTANCE:-testnet-instance}"
BLOCKSCOUT_DB_FRESH_NAME="${BLOCKSCOUT_DB_FRESH_NAME:-blockscout_$(date -u +%Y%m%d_%H%M%S)}"
BLOCKSCOUT_DB_CHARSET="${BLOCKSCOUT_DB_CHARSET:-UTF8}"
BLOCKSCOUT_DB_COLLATION="${BLOCKSCOUT_DB_COLLATION:-en_US.UTF8}"
BLOCKSCOUT_SECRET_MANAGER_SECRET="${BLOCKSCOUT_SECRET_MANAGER_SECRET:-blockscout-database-secret}"
BLOCKSCOUT_EXTERNAL_SECRET="${BLOCKSCOUT_EXTERNAL_SECRET:-evm-secret-key}"
BLOCKSCOUT_EXTERNAL_SECRET_SYNC_TIMEOUT_SECONDS="${BLOCKSCOUT_EXTERNAL_SECRET_SYNC_TIMEOUT_SECONDS:-300}"

BLOCKSCOUT_REDIS_RESET_POD="${BLOCKSCOUT_REDIS_RESET_POD:-blockscout-redis-reset}"
BLOCKSCOUT_REDIS_RESET_IMAGE="${BLOCKSCOUT_REDIS_RESET_IMAGE:-redis:7-alpine}"
BLOCKSCOUT_REDIS_FLUSH_COMMAND="${BLOCKSCOUT_REDIS_FLUSH_COMMAND:-FLUSHDB}"
BLOCKSCOUT_REDIS_TIMEOUT_SECONDS="${BLOCKSCOUT_REDIS_TIMEOUT_SECONDS:-10}"
BLOCKSCOUT_REDIS_REQUIRED="${BLOCKSCOUT_REDIS_REQUIRED:-false}"
BLOCKSCOUT_REDIS_URL="${BLOCKSCOUT_REDIS_URL:-redis://10.96.88.3:6379/0}"
BLOCKSCOUT_REDIS_URL_SECRET="${BLOCKSCOUT_REDIS_URL_SECRET:-}"
BLOCKSCOUT_REDIS_URL_SECRET_KEY="${BLOCKSCOUT_REDIS_URL_SECRET_KEY:-}"
DRY_RUN="${DRY_RUN:-false}"

CONFIRM_VALUE="run-testnet-evm-migration"

require_env() {
  local name="$1"
  if [[ -z "${!name:-}" ]]; then
    echo "Missing required environment variable: ${name}" >&2
    exit 1
  fi
}

kubectl_ns() {
  "${KUBECTL}" -n "${NAMESPACE}" "$@"
}

wait_for_pod_deleted() {
  local pod="$1"
  if kubectl_ns get "pod/${pod}" >/dev/null 2>&1; then
    kubectl_ns wait --for=delete "pod/${pod}" --timeout=10m
  fi
}

delete_pod_if_exists() {
  local pod="$1"
  kubectl_ns delete "pod/${pod}" --ignore-not-found --wait=true
}

discover_blockscout_resources() {
  kubectl_ns get deploy,statefulset -o name \
    | grep -E "${BLOCKSCOUT_RESOURCE_REGEX}" \
    | sort
}

save_blockscout_replicas() {
  local replicas_file="$1"
  : > "${replicas_file}"
  while IFS= read -r resource; do
    [[ -n "${resource}" ]] || continue
    local replicas
    replicas="$(kubectl_ns get "${resource}" -o jsonpath='{.spec.replicas}')"
    echo "${resource} ${replicas:-0}" >> "${replicas_file}"
  done < <(discover_blockscout_resources)

  if [[ ! -s "${replicas_file}" ]]; then
    echo "No Blockscout deployments/statefulsets matched regex: ${BLOCKSCOUT_RESOURCE_REGEX}" >&2
    exit 1
  fi
}

scale_resources_from_file() {
  local replicas_file="$1"
  local replicas="$2"
  while read -r resource _original_replicas; do
    [[ -n "${resource}" ]] || continue
    kubectl_ns scale "${resource}" --replicas="${replicas}"
  done < "${replicas_file}"
}

restore_resources_from_file() {
  local replicas_file="$1"
  while read -r resource replicas; do
    [[ -n "${resource}" ]] || continue
    kubectl_ns scale "${resource}" --replicas="${replicas}"
  done < "${replicas_file}"
}

is_dry_run() {
  [[ "${DRY_RUN}" == "1" || "${DRY_RUN}" == "true" || "${DRY_RUN}" == "yes" ]]
}

should_pin_payy_evm_image() {
  [[ "${PAYY_EVM_PIN_IMAGE}" == "1" ||
    "${PAYY_EVM_PIN_IMAGE}" == "true" ||
    "${PAYY_EVM_PIN_IMAGE}" == "yes" ]]
}

should_patch_payy_evm_run_subcommand() {
  [[ "${PAYY_EVM_PATCH_RUN_SUBCOMMAND}" == "1" ||
    "${PAYY_EVM_PATCH_RUN_SUBCOMMAND}" == "true" ||
    "${PAYY_EVM_PATCH_RUN_SUBCOMMAND}" == "yes" ]]
}

is_blockscout_redis_required() {
  [[ "${BLOCKSCOUT_REDIS_REQUIRED}" == "1" ||
    "${BLOCKSCOUT_REDIS_REQUIRED}" == "true" ||
    "${BLOCKSCOUT_REDIS_REQUIRED}" == "yes" ]]
}

database_name_from_url() {
  local url="$1"
  local base_url="${url%%\?*}"
  echo "${base_url##*/}"
}

database_url_with_name() {
  local url="$1"
  local database="$2"
  local base_url="${url%%\?*}"
  local query=""
  if [[ "${url}" == *\?* ]]; then
    query="?${url#*\?}"
  fi

  echo "${base_url%/*}/${database}${query}"
}

image_repository() {
  local image="$1"
  local without_digest="${image%@*}"
  local last_component="${without_digest##*/}"
  if [[ "${last_component}" == *:* ]]; then
    echo "${without_digest%:*}"
  else
    echo "${without_digest}"
  fi
}

image_tag() {
  local image="$1"
  if [[ "${image}" == *@* ]]; then
    echo ""
    return
  fi

  local last_component="${image##*/}"
  if [[ "${last_component}" == *:* ]]; then
    echo "${last_component##*:}"
  else
    echo "latest"
  fi
}

normalize_digest() {
  local digest="$1"
  digest="${digest%%$'\n'*}"
  if [[ "${digest}" == *@sha256:* ]]; then
    digest="${digest##*@}"
  fi
  if [[ "${digest}" == sha256:* ]]; then
    echo "${digest}"
  else
    echo "sha256:${digest}"
  fi
}

resolve_image_digest() {
  local image="$1"
  local digest=""

  if command -v gcloud >/dev/null 2>&1; then
    digest="$(gcloud container images describe "${image}" \
      --format='value(image_summary.digest)' 2>/dev/null || true)"
    if [[ -z "${digest}" ]]; then
      local repository tag
      repository="$(image_repository "${image}")"
      tag="$(image_tag "${image}")"
      if [[ -n "${tag}" ]]; then
        digest="$(gcloud container images list-tags "${repository}" \
          --filter="tags:${tag}" \
          --format='value(digest)' \
          --limit=1 2>/dev/null | head -n 1 || true)"
      fi
    fi
  fi

  if [[ -z "${digest}" ]] && command -v crane >/dev/null 2>&1; then
    digest="$(crane digest "${image}" 2>/dev/null || true)"
  fi

  if [[ -z "${digest}" ]]; then
    echo "Could not resolve immutable digest for ${image}." >&2
    echo "Install/authenticate gcloud or crane, or pass PAYY_EVM_IMAGE as repo@sha256:digest." >&2
    exit 1
  fi

  normalize_digest "${digest}"
}

pin_payy_evm_image() {
  if ! should_pin_payy_evm_image; then
    return
  fi

  if [[ "${PAYY_EVM_IMAGE_SOURCE}" == *@sha256:* ]]; then
    PAYY_EVM_IMAGE="${PAYY_EVM_IMAGE_SOURCE}"
    PAYY_EVM_IMAGE_DIGEST="${PAYY_EVM_IMAGE_SOURCE##*@}"
    return
  fi

  local repository
  repository="$(image_repository "${PAYY_EVM_IMAGE_SOURCE}")"
  PAYY_EVM_IMAGE_DIGEST="$(resolve_image_digest "${PAYY_EVM_IMAGE_SOURCE}")"
  PAYY_EVM_IMAGE="${repository}@${PAYY_EVM_IMAGE_DIGEST}"
}

wait_for_selector_deleted() {
  local selector="$1"
  [[ -n "${selector}" ]] || return 0

  while kubectl_ns get pods -l "${selector}" --no-headers 2>/dev/null | grep -q .; do
    sleep 5
  done
}

run_pod_to_completion() {
  local pod="$1"
  local timeout="$2"

  local deadline
  deadline=$((SECONDS + 600))
  while true; do
    local phase
    phase="$(kubectl_ns get "pod/${pod}" -o jsonpath='{.status.phase}' 2>/dev/null || true)"
    case "${phase}" in
      Running | Succeeded | Failed)
        break
        ;;
      "")
        ;;
      Pending)
        ;;
      *)
        echo "Pod ${pod} phase: ${phase}"
        ;;
    esac
    if (( SECONDS > deadline )); then
      kubectl_ns describe "pod/${pod}" || true
      echo "Timed out waiting for pod ${pod} to start" >&2
      exit 1
    fi
    sleep 2
  done

  kubectl_ns logs -f "pod/${pod}" --pod-running-timeout="${timeout}" || true

  while true; do
    local phase
    phase="$(kubectl_ns get "pod/${pod}" -o jsonpath='{.status.phase}')"
    case "${phase}" in
      Succeeded | Failed)
        break
        ;;
    esac
    sleep 5
  done

  local phase
  phase="$(kubectl_ns get "pod/${pod}" -o jsonpath='{.status.phase}')"
  if [[ "${phase}" != "Succeeded" ]]; then
    kubectl_ns describe "pod/${pod}" || true
    echo "Pod ${pod} finished with phase ${phase}" >&2
    exit 1
  fi
}

create_migration_pod() {
  delete_pod_if_exists "${MIGRATION_POD}"
  print_migration_pod_manifest | kubectl_ns apply -f -
}

create_blockscout_db_reset_pod() {
  delete_pod_if_exists "${BLOCKSCOUT_DB_RESET_POD}"
  cat <<YAML | kubectl_ns apply -f -
apiVersion: v1
kind: Pod
metadata:
  name: ${BLOCKSCOUT_DB_RESET_POD}
  labels:
    app.kubernetes.io/name: blockscout-db-reset
spec:
  restartPolicy: Never
  containers:
    - name: reset-db
      image: ${BLOCKSCOUT_DB_RESET_IMAGE}
      imagePullPolicy: IfNotPresent
      env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: ${BLOCKSCOUT_DB_SECRET}
              key: ${BLOCKSCOUT_DB_SECRET_KEY}
      command:
        - /bin/sh
        - -ceu
        - |
          DB_NAME="\$(printf '%s' "\${DATABASE_URL}" | sed -E 's#^([^?]*://[^/]+/)([^?]+)(\\?.*)?\$#\\2#')"
          ADMIN_URL="\$(printf '%s' "\${DATABASE_URL}" | sed -E 's#^([^?]*://[^/]+/)([^?]+)(\\?.*)?\$#\\1postgres\\3#')"
          if [ -z "\${DB_NAME}" ] || [ "\${DB_NAME}" = "\${DATABASE_URL}" ]; then
            echo "Could not parse database name from DATABASE_URL" >&2
            exit 1
          fi
          DB_LITERAL="'\$(printf '%s' "\${DB_NAME}" | sed "s/'/''/g")'"
          DB_IDENTIFIER="\"\$(printf '%s' "\${DB_NAME}" | sed 's/"/""/g')\""
          psql "\${ADMIN_URL}" -v ON_ERROR_STOP=1 \
            -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = \${DB_LITERAL} AND pid <> pg_backend_pid();" \
            -c "DROP DATABASE IF EXISTS \${DB_IDENTIFIER};" \
            -c "CREATE DATABASE \${DB_IDENTIFIER};"
YAML
}

wait_for_blockscout_database_secret() {
  local expected_database="$1"
  local deadline=$((SECONDS + BLOCKSCOUT_EXTERNAL_SECRET_SYNC_TIMEOUT_SECONDS))

  while true; do
    local current_url current_database
    current_url="$(kubectl_ns get "secret/${BLOCKSCOUT_DB_SECRET}" \
      -o "jsonpath={.data.${BLOCKSCOUT_DB_SECRET_KEY}}" | base64 -d)"
    current_database="$(database_name_from_url "${current_url}")"
    if [[ "${current_database}" == "${expected_database}" ]]; then
      return
    fi
    if (( SECONDS > deadline )); then
      echo "Timed out waiting for ${BLOCKSCOUT_DB_SECRET} to sync ${expected_database}" >&2
      exit 1
    fi
    sleep 2
  done
}

reset_blockscout_database_with_fresh_cloudsql() {
  local current_url new_url
  current_url="$(kubectl_ns get "secret/${BLOCKSCOUT_DB_SECRET}" \
    -o "jsonpath={.data.${BLOCKSCOUT_DB_SECRET_KEY}}" | base64 -d)"
  new_url="$(database_url_with_name "${current_url}" "${BLOCKSCOUT_DB_FRESH_NAME}")"

  echo "Creating fresh Blockscout database ${BLOCKSCOUT_DB_FRESH_NAME}..."
  gcloud sql databases create "${BLOCKSCOUT_DB_FRESH_NAME}" \
    --instance="${BLOCKSCOUT_CLOUDSQL_INSTANCE}" \
    --project="${BLOCKSCOUT_CLOUDSQL_PROJECT}" \
    --charset="${BLOCKSCOUT_DB_CHARSET}" \
    --collation="${BLOCKSCOUT_DB_COLLATION}"

  echo "Writing new Blockscout DATABASE_URL to Secret Manager..."
  printf '%s' "${new_url}" | gcloud secrets versions add "${BLOCKSCOUT_SECRET_MANAGER_SECRET}" \
    --project="${BLOCKSCOUT_CLOUDSQL_PROJECT}" \
    --data-file=-

  kubectl_ns annotate "externalsecret/${BLOCKSCOUT_EXTERNAL_SECRET}" \
    "force-sync=$(date +%s)" \
    --overwrite
  wait_for_blockscout_database_secret "${BLOCKSCOUT_DB_FRESH_NAME}"
}

reset_blockscout_database() {
  case "${BLOCKSCOUT_DB_RESET_MODE}" in
    fresh-cloud-sql)
      reset_blockscout_database_with_fresh_cloudsql
      ;;
    drop-create-pod)
      create_blockscout_db_reset_pod
      run_pod_to_completion "${BLOCKSCOUT_DB_RESET_POD}" "30m"
      delete_pod_if_exists "${BLOCKSCOUT_DB_RESET_POD}"
      ;;
    *)
      echo "BLOCKSCOUT_DB_RESET_MODE must be fresh-cloud-sql or drop-create-pod" >&2
      exit 1
      ;;
  esac
}

create_blockscout_redis_reset_pod() {
  delete_pod_if_exists "${BLOCKSCOUT_REDIS_RESET_POD}"

  local redis_env
  if [[ -n "${BLOCKSCOUT_REDIS_URL_SECRET}" ]]; then
    if [[ -z "${BLOCKSCOUT_REDIS_URL_SECRET_KEY}" ]]; then
      echo "BLOCKSCOUT_REDIS_URL_SECRET_KEY is required when BLOCKSCOUT_REDIS_URL_SECRET is set" >&2
      exit 1
    fi
    redis_env=$(cat <<YAML
        - name: REDIS_URL
          valueFrom:
            secretKeyRef:
              name: ${BLOCKSCOUT_REDIS_URL_SECRET}
              key: ${BLOCKSCOUT_REDIS_URL_SECRET_KEY}
YAML
)
  else
    require_env BLOCKSCOUT_REDIS_URL
    redis_env=$(cat <<YAML
        - name: REDIS_URL
          value: "${BLOCKSCOUT_REDIS_URL}"
YAML
)
  fi

  cat <<YAML | kubectl_ns apply -f -
apiVersion: v1
kind: Pod
metadata:
  name: ${BLOCKSCOUT_REDIS_RESET_POD}
  labels:
    app.kubernetes.io/name: blockscout-redis-reset
spec:
  restartPolicy: Never
  containers:
    - name: reset-redis
      image: ${BLOCKSCOUT_REDIS_RESET_IMAGE}
      imagePullPolicy: IfNotPresent
      env:
${redis_env}
        - name: REDIS_FLUSH_COMMAND
          value: "${BLOCKSCOUT_REDIS_FLUSH_COMMAND}"
      command:
        - /bin/sh
        - -ceu
        - |
          case "\${REDIS_FLUSH_COMMAND}" in
            FLUSHDB|FLUSHALL) ;;
            *) echo "REDIS_FLUSH_COMMAND must be FLUSHDB or FLUSHALL" >&2; exit 1 ;;
          esac
          timeout "${BLOCKSCOUT_REDIS_TIMEOUT_SECONDS}" redis-cli -u "\${REDIS_URL}" "\${REDIS_FLUSH_COMMAND}"
YAML
}

reset_blockscout_redis() {
  create_blockscout_redis_reset_pod

  local deadline=$((SECONDS + BLOCKSCOUT_REDIS_TIMEOUT_SECONDS + 120))
  local phase=""
  while true; do
    phase="$(kubectl_ns get "pod/${BLOCKSCOUT_REDIS_RESET_POD}" \
      -o jsonpath='{.status.phase}' 2>/dev/null || true)"
    case "${phase}" in
      Succeeded | Failed)
        break
        ;;
    esac
    if (( SECONDS > deadline )); then
      phase="TimedOut"
      break
    fi
    sleep 2
  done

  kubectl_ns logs "pod/${BLOCKSCOUT_REDIS_RESET_POD}" || true
  delete_pod_if_exists "${BLOCKSCOUT_REDIS_RESET_POD}"

  if [[ "${phase}" == "Succeeded" ]]; then
    return
  fi
  if is_blockscout_redis_required; then
    echo "Blockscout Redis reset failed with phase ${phase}" >&2
    exit 1
  fi

  echo "WARNING: Blockscout Redis reset failed with phase ${phase}; continuing because BLOCKSCOUT_REDIS_REQUIRED=false." >&2
}

preflight() {
  if ! is_dry_run && [[ "${CONFIRM:-}" != "${CONFIRM_VALUE}" ]]; then
    echo "Set CONFIRM=${CONFIRM_VALUE} to run this destructive testnet migration." >&2
    exit 1
  fi

  pin_payy_evm_image

  "${KUBECTL}" version --client >/dev/null
  "${KUBECTL}" get namespace "${NAMESPACE}" >/dev/null
  kubectl_ns get "statefulset/${PAYY_EVM_STATEFULSET}" >/dev/null
  kubectl_ns get "pvc/${PAYY_EVM_PVC}" >/dev/null
  kubectl_ns get "secret/${BLOCKSCOUT_DB_SECRET}" >/dev/null
  case "${BLOCKSCOUT_DB_RESET_MODE}" in
    fresh-cloud-sql)
      command -v gcloud >/dev/null
      kubectl_ns get "externalsecret/${BLOCKSCOUT_EXTERNAL_SECRET}" >/dev/null
      gcloud sql instances describe "${BLOCKSCOUT_CLOUDSQL_INSTANCE}" \
        --project="${BLOCKSCOUT_CLOUDSQL_PROJECT}" >/dev/null
      gcloud secrets describe "${BLOCKSCOUT_SECRET_MANAGER_SECRET}" \
        --project="${BLOCKSCOUT_CLOUDSQL_PROJECT}" >/dev/null
      ;;
    drop-create-pod)
      ;;
    *)
      echo "BLOCKSCOUT_DB_RESET_MODE must be fresh-cloud-sql or drop-create-pod" >&2
      exit 1
      ;;
  esac
  if [[ -n "${BLOCKSCOUT_REDIS_URL_SECRET}" ]]; then
    kubectl_ns get "secret/${BLOCKSCOUT_REDIS_URL_SECRET}" >/dev/null
  else
    require_env BLOCKSCOUT_REDIS_URL
  fi

  echo "kubectl context: $("${KUBECTL}" config current-context)"
  echo "namespace: ${NAMESPACE}"
  echo "payy-evm statefulset: ${PAYY_EVM_STATEFULSET}"
  echo "payy-evm pvc: ${PAYY_EVM_PVC}"
  echo "payy-evm image source: ${PAYY_EVM_IMAGE_SOURCE}"
  echo "payy-evm image resolved: ${PAYY_EVM_IMAGE}"
  echo "payy-evm patch run subcommand: ${PAYY_EVM_PATCH_RUN_SUBCOMMAND}"
  echo "blockscout resource regex: ${BLOCKSCOUT_RESOURCE_REGEX}"
  echo "blockscout db reset mode: ${BLOCKSCOUT_DB_RESET_MODE}"
}

print_dry_run_plan() {
  local blockscout_replicas="$1"

  echo
  echo "DRY RUN: no Kubernetes resources will be mutated."
  echo
  echo "Resolved variables:"
  cat <<EOF
KUBECTL=${KUBECTL}
NAMESPACE=${NAMESPACE}
PAYY_EVM_STATEFULSET=${PAYY_EVM_STATEFULSET}
PAYY_EVM_CONTAINER=${PAYY_EVM_CONTAINER}
PAYY_EVM_PVC=${PAYY_EVM_PVC}
PAYY_EVM_DATADIR=${PAYY_EVM_DATADIR}
PAYY_EVM_IMAGE_SOURCE=${PAYY_EVM_IMAGE_SOURCE}
PAYY_EVM_PIN_IMAGE=${PAYY_EVM_PIN_IMAGE}
PAYY_EVM_IMAGE=${PAYY_EVM_IMAGE}
PAYY_EVM_IMAGE_DIGEST=${PAYY_EVM_IMAGE_DIGEST}
PAYY_EVM_PATCH_RUN_SUBCOMMAND=${PAYY_EVM_PATCH_RUN_SUBCOMMAND}
MIGRATION_POD=${MIGRATION_POD}
MIGRATION_TIMEOUT=${MIGRATION_TIMEOUT}
PAYY_EVM_READY_TIMEOUT=${PAYY_EVM_READY_TIMEOUT}
BLOCKSCOUT_RESOURCE_REGEX=${BLOCKSCOUT_RESOURCE_REGEX}
BLOCKSCOUT_SELECTOR=${BLOCKSCOUT_SELECTOR}
BLOCKSCOUT_DB_RESET_MODE=${BLOCKSCOUT_DB_RESET_MODE}
BLOCKSCOUT_DB_SECRET=${BLOCKSCOUT_DB_SECRET}
BLOCKSCOUT_DB_SECRET_KEY=${BLOCKSCOUT_DB_SECRET_KEY}
BLOCKSCOUT_DB_RESET_POD=${BLOCKSCOUT_DB_RESET_POD}
BLOCKSCOUT_DB_RESET_IMAGE=${BLOCKSCOUT_DB_RESET_IMAGE}
BLOCKSCOUT_CLOUDSQL_PROJECT=${BLOCKSCOUT_CLOUDSQL_PROJECT}
BLOCKSCOUT_CLOUDSQL_INSTANCE=${BLOCKSCOUT_CLOUDSQL_INSTANCE}
BLOCKSCOUT_DB_FRESH_NAME=${BLOCKSCOUT_DB_FRESH_NAME}
BLOCKSCOUT_DB_CHARSET=${BLOCKSCOUT_DB_CHARSET}
BLOCKSCOUT_DB_COLLATION=${BLOCKSCOUT_DB_COLLATION}
BLOCKSCOUT_SECRET_MANAGER_SECRET=${BLOCKSCOUT_SECRET_MANAGER_SECRET}
BLOCKSCOUT_EXTERNAL_SECRET=${BLOCKSCOUT_EXTERNAL_SECRET}
BLOCKSCOUT_EXTERNAL_SECRET_SYNC_TIMEOUT_SECONDS=${BLOCKSCOUT_EXTERNAL_SECRET_SYNC_TIMEOUT_SECONDS}
BLOCKSCOUT_REDIS_RESET_POD=${BLOCKSCOUT_REDIS_RESET_POD}
BLOCKSCOUT_REDIS_RESET_IMAGE=${BLOCKSCOUT_REDIS_RESET_IMAGE}
BLOCKSCOUT_REDIS_FLUSH_COMMAND=${BLOCKSCOUT_REDIS_FLUSH_COMMAND}
BLOCKSCOUT_REDIS_TIMEOUT_SECONDS=${BLOCKSCOUT_REDIS_TIMEOUT_SECONDS}
BLOCKSCOUT_REDIS_REQUIRED=${BLOCKSCOUT_REDIS_REQUIRED}
BLOCKSCOUT_REDIS_URL_SECRET=${BLOCKSCOUT_REDIS_URL_SECRET}
BLOCKSCOUT_REDIS_URL_SECRET_KEY=${BLOCKSCOUT_REDIS_URL_SECRET_KEY}
BLOCKSCOUT_REDIS_URL=${BLOCKSCOUT_REDIS_URL:+<set>}
EOF

  echo
  echo "Matched Blockscout workloads and current replicas:"
  cat "${blockscout_replicas}"

  echo
  echo "Migration pod manifest:"
  print_migration_pod_manifest

  echo
  echo "Planned operations:"
  cat <<EOF
1. kubectl -n ${NAMESPACE} scale statefulset/${PAYY_EVM_STATEFULSET} --replicas=0
2. Wait for pod/${PAYY_EVM_STATEFULSET}-0 to delete.
3. Scale matched Blockscout workloads to 0.
4. Wait for pods matching ${BLOCKSCOUT_SELECTOR} to delete.
5. Create pod/${MIGRATION_POD} with PVC ${PAYY_EVM_PVC} mounted at ${PAYY_EVM_DATADIR}.
6. Wait for pod/${MIGRATION_POD} to complete successfully, then delete it.
7. Reset Blockscout DB using mode ${BLOCKSCOUT_DB_RESET_MODE}.
8. Create pod/${BLOCKSCOUT_REDIS_RESET_POD} to run ${BLOCKSCOUT_REDIS_FLUSH_COMMAND} against Blockscout Redis.
9. kubectl -n ${NAMESPACE} set image statefulset/${PAYY_EVM_STATEFULSET} ${PAYY_EVM_CONTAINER}=${PAYY_EVM_IMAGE}
10. Ensure container ${PAYY_EVM_CONTAINER} args start with run when PAYY_EVM_PATCH_RUN_SUBCOMMAND=true.
11. kubectl -n ${NAMESPACE} scale statefulset/${PAYY_EVM_STATEFULSET} --replicas=1
12. Restore matched Blockscout workloads to their original replica counts.
EOF
}

payy_evm_container_index() {
  local index=0
  local name
  while IFS= read -r name; do
    if [[ "${name}" == "${PAYY_EVM_CONTAINER}" ]]; then
      echo "${index}"
      return
    fi
    index=$((index + 1))
  done < <(kubectl_ns get "statefulset/${PAYY_EVM_STATEFULSET}" \
    -o jsonpath='{range .spec.template.spec.containers[*]}{.name}{"\n"}{end}')

  echo "Container ${PAYY_EVM_CONTAINER} not found in statefulset/${PAYY_EVM_STATEFULSET}" >&2
  exit 1
}

payy_evm_first_arg_is_run() {
  local container_index="$1"
  local first_arg
  first_arg="$(kubectl_ns get "statefulset/${PAYY_EVM_STATEFULSET}" \
    -o "jsonpath={.spec.template.spec.containers[${container_index}].args[0]}" \
    2>/dev/null || true)"
  [[ "${first_arg}" == "run" ]]
}

ensure_payy_evm_run_subcommand() {
  if ! should_patch_payy_evm_run_subcommand; then
    return
  fi

  local container_index
  container_index="$(payy_evm_container_index)"
  if payy_evm_first_arg_is_run "${container_index}"; then
    return
  fi

  local prepend_patch
  prepend_patch="$(printf '[{"op":"add","path":"/spec/template/spec/containers/%s/args/0","value":"run"}]' \
    "${container_index}")"
  if kubectl_ns patch "statefulset/${PAYY_EVM_STATEFULSET}" \
    --type=json \
    -p="${prepend_patch}" >/dev/null 2>&1; then
    return
  fi

  local create_args_patch
  create_args_patch="$(printf '[{"op":"add","path":"/spec/template/spec/containers/%s/args","value":["run"]}]' \
    "${container_index}")"
  kubectl_ns patch "statefulset/${PAYY_EVM_STATEFULSET}" \
    --type=json \
    -p="${create_args_patch}"
}

print_migration_pod_manifest() {
  cat <<YAML
apiVersion: v1
kind: Pod
metadata:
  name: ${MIGRATION_POD}
  labels:
    app.kubernetes.io/name: payy-evm-volume-migration
spec:
  restartPolicy: Never
  terminationGracePeriodSeconds: 30
  securityContext:
    fsGroup: 1001
    fsGroupChangePolicy: OnRootMismatch
  containers:
    - name: migrate
      image: ${PAYY_EVM_IMAGE}
      imagePullPolicy: Always
      command:
        - /usr/bin/payy-evm
      args:
        - --log-format
        - MINIMAL
        - migrate
        - --network
        - testnet
        - --datadir
        - ${PAYY_EVM_DATADIR}
        - --skip-empty-blocks
      securityContext:
        runAsUser: 1001
        runAsGroup: 1001
        runAsNonRoot: true
      volumeMounts:
        - name: data
          mountPath: ${PAYY_EVM_DATADIR}
  volumes:
    - name: data
      persistentVolumeClaim:
        claimName: ${PAYY_EVM_PVC}
YAML
}

main() {
  preflight

  local blockscout_replicas
  blockscout_replicas="$(mktemp)"
  save_blockscout_replicas "${blockscout_replicas}"

  if is_dry_run; then
    print_dry_run_plan "${blockscout_replicas}"
    return
  fi

  echo "Scaling payy-evm to 0..."
  kubectl_ns scale "statefulset/${PAYY_EVM_STATEFULSET}" --replicas=0
  wait_for_pod_deleted "${PAYY_EVM_STATEFULSET}-0"

  echo "Scaling Blockscout resources to 0..."
  scale_resources_from_file "${blockscout_replicas}" 0
  wait_for_selector_deleted "${BLOCKSCOUT_SELECTOR}"

  echo "Running payy-evm migration pod..."
  create_migration_pod
  run_pod_to_completion "${MIGRATION_POD}" "${MIGRATION_TIMEOUT}"
  delete_pod_if_exists "${MIGRATION_POD}"

  echo "Resetting Blockscout PostgreSQL database..."
  reset_blockscout_database

  echo "Resetting Blockscout Redis..."
  reset_blockscout_redis

  echo "Updating payy-evm image and scaling to 1..."
  kubectl_ns set image "statefulset/${PAYY_EVM_STATEFULSET}" \
    "${PAYY_EVM_CONTAINER}=${PAYY_EVM_IMAGE}"
  ensure_payy_evm_run_subcommand
  kubectl_ns scale "statefulset/${PAYY_EVM_STATEFULSET}" --replicas=1
  kubectl_ns rollout status "statefulset/${PAYY_EVM_STATEFULSET}" \
    --timeout="${PAYY_EVM_READY_TIMEOUT}"

  echo "Restoring Blockscout resources to previous replica counts..."
  restore_resources_from_file "${blockscout_replicas}"

  echo "Migration complete."
  echo "Blockscout resources restored from:"
  cat "${blockscout_replicas}"
}

main "$@"
