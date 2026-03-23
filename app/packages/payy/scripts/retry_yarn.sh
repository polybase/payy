# Retry `yarn install` with exponential backoff during eas build pre-install
try_yarn_install() {
  local ATTEMPT=1
  local MAX_ATTEMPTS=5
  local DELAY=5 # Delays - 5, 10, 20, 40, 80 seconds max
  local EXP_BACKOFF=2

  until yarn install --network-timeout 1000000000; do
    echo "yarn install failed: ${ATTEMPT} of ${MAX_ATTEMPTS} attempts"

    if [[ "${ATTEMPT}" -gt "${MAX_ATTEMPTS}" ]]; then
      echo "yarn install still failing after ${MAX_ATTEMPTS} attempts. Aborting..."
      exit 1
    fi

    echo "Sleeping for ${DELAY} second(s) before retrying..."
    sleep ${DELAY}
    ATTEMPT=$(( ATTEMPT + 1 ))
    DELAY=$(( DELAY * EXP_BACKOFF ))
    # cap max limit (extra safety - `ATTEMPTS` should ensure that the delay doesn't grow
    # indefinitely)
    DELAY=$(( DELAY > 80 ? 80 : DELAY ))
  done

  echo "yarn install succeeded in ${ATTEMPT} attempt(s)"
}