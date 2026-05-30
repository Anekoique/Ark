# Build source for the `ark sandbox` image.
#
# CI builds and pushes this to `ghcr.io/anekoique/ark-sandbox:<ark-version>`
# on each release; a release MUST ship its matching tag or `ark sandbox
# create` fails with ImagePullFailed (the binary's default image is the
# version-matched tag). `ark sandbox` PULLS this image — it never builds it.
#
# The container runs as a fixed `ark-sandbox` user (uid 2000) baked into the
# image, so /etc/passwd is well-formed and tools that read it (bash prompt,
# git, claude CLI) see a real name. uid 2000 is picked to side-step the
# `node` user the base image ships at uid 1000. Workspace files written from
# inside the box are owned by uid 2000 on the host; that's not your shell
# user's uid, but the host bind mount keeps them readable and editable
# regardless of literal ownership.

FROM node:22-bookworm-slim

# Agent CLIs available in-box. Pin as needed.
RUN npm install -g \
        @anthropic-ai/claude-code \
        @openai/codex \
    && npm cache clean --force

# Git + basics the in-box workflow needs.
RUN apt-get update \
    && apt-get install -y --no-install-recommends git ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

# A fixed sandbox user so `whoami`, `$USER`, bash's prompt, and tools that
# read the passwd db (git, claude) all see a real name. Owns /workspace and
# its own writable HOME. The base image (`node:*-slim`) ships a `node` user
# at uid 1000 we don't need, so remove it first; pick uid 2000 to side-step
# any future collision.
RUN userdel -r node 2>/dev/null || true \
    && useradd -m -u 2000 -s /bin/bash ark-sandbox \
    && mkdir -p /workspace \
    && chown ark-sandbox:ark-sandbox /workspace

# `/workspace` is bind-mounted from the host where files are owned by the
# host user (a different uid than ark-sandbox inside the box). Git's safety
# check refuses to operate on a repo it doesn't own; mark it safe via the
# system-wide config (`/etc/gitconfig`) so the rule survives a bind-mount of
# the host's `~/.gitconfig` over `$HOME/.gitconfig` under share_host_config.
RUN git config --system --add safe.directory /workspace

USER ark-sandbox

# `USER` is set explicitly because `docker exec` does not run a login shell
# and so doesn't populate it via PAM; tools that key off `$USER` (claude's
# config path, bash `\u`) need this set. `PS1` shapes the default prompt.
ENV HOME=/home/ark-sandbox \
    USER=ark-sandbox \
    PS1='\u@ark-sandbox:\w\$ '

WORKDIR /workspace
