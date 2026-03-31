#!/bin/sh
set -e

# Stop any running assistant processes gracefully before removal.
# On Alpine with OpenRC, services are managed by the user — nothing to do here
# automatically. If a user registered custom OpenRC services they should stop
# them manually before removing this package.
