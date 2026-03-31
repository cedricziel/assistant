#!/bin/sh
set -e

cat <<'MSG'

assistant has been installed.  To run a chat bot:

  1. Make sure ~/.assistant/config.toml contains your credentials.

  2. Run the assistant directly:

       assistant orchestrator run
       assistant webui serve

  3. To run as a background OpenRC service, install a custom init script.
     Example for the web UI:

       cat > /etc/init.d/assistant-web-ui << 'EOF'
       #!/sbin/openrc-run
       name="assistant-web-ui"
       command="/usr/local/bin/assistant"
       command_args="webui serve"
       command_user="$(id -un)"
       pidfile="/run/${RC_SVCNAME}.pid"
       command_background=yes
       EOF
       chmod +x /etc/init.d/assistant-web-ui
       rc-update add assistant-web-ui default
       rc-service assistant-web-ui start

MSG
