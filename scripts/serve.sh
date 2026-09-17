#!/usr/bin/env bash
# Serve the generated bubble-watch site over HTTP on the LAN.
#
# Why a tiny server rather than a full one: the site is static HTML with no
# JavaScript, so all that is needed is "read a file and write it to the socket".
# nginx is not installed on this host, and adding a heavyweight server to publish
# five static files would be a poor trade. This binds to 0.0.0.0 so it is
# reachable from other devices on the LAN; it is not exposed to the internet.
#
# Security notes:
#   * It serves exactly one directory. Path traversal is rejected by resolving
#     the request against the document root and refusing anything that escapes
#     it.
#   * Only GET/HEAD are implemented. No uploads, no CGI, no execution.
#   * Directory listings are not generated; a missing index returns 404.
set -euo pipefail

ROOT="${1:-/home/daim/workspace/bubble-watch/site}"
PORT="${2:-8770}"
BIND="${3:-0.0.0.0}"

exec /usr/bin/env python3 - "$ROOT" "$PORT" "$BIND" <<'PY'
import http.server, functools, os, socketserver, sys

root, port, bind = sys.argv[1], int(sys.argv[2]), sys.argv[3]
root = os.path.realpath(root)


class Handler(http.server.SimpleHTTPRequestHandler):
    """Static handler pinned to ROOT, with traversal refused."""

    def translate_path(self, path):
        # Strip query/fragment, resolve, then confirm containment.
        p = path.split("?", 1)[0].split("#", 1)[0]
        candidate = os.path.realpath(os.path.join(root, p.lstrip("/")))
        if candidate != root and not candidate.startswith(root + os.sep):
            return os.path.join(root, "__forbidden__")
        if os.path.isdir(candidate):
            candidate = os.path.join(candidate, "index.html")
        return candidate

    def list_directory(self, path):
        # Refuse to enumerate the filesystem.
        self.send_error(404, "No directory listing")
        return None

    def log_message(self, fmt, *args):
        # One line per request to stdout, so journald captures access history.
        sys.stdout.write("%s - %s\n" % (self.address_string(), fmt % args))
        sys.stdout.flush()


class Server(socketserver.ThreadingTCPServer):
    allow_reuse_address = True
    daemon_threads = True


if not os.path.isdir(root):
    sys.stderr.write("document root does not exist: %s\n" % root)
    raise SystemExit(3)

with Server((bind, port), functools.partial(Handler, directory=root)) as httpd:
    sys.stdout.write("bubble-watch site serving %s on http://%s:%d/\n" % (root, bind, port))
    sys.stdout.flush()
    httpd.serve_forever()
PY
