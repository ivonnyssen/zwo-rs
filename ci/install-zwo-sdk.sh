#!/usr/bin/env bash
#
# Install the ZWO ASI Camera + EFW filter wheel SDK (MIT-licensed) from the INDI
# indi-3rdparty mirror, so the crate can link (`cargo build`/`test`).
#
# INDI vendors ZWO's upstream prebuilt shared libraries as `*.bin`; we install
# them under $PREFIX/lib with the linker name (`lib<name>.so`) plus headers under
# $PREFIX/include, then refresh the loader cache.
#
# Env:
#   PREFIX   install prefix (default: /usr/local)
#   REF      indi-3rdparty git ref (default: master)
set -euo pipefail

PREFIX="${PREFIX:-/usr/local}"
REF="${REF:-master}"
BASE="https://github.com/indilib/indi-3rdparty/raw/${REF}/libasi"

case "$(uname -m)" in
  x86_64 | amd64) ARCH=x64 ;;
  aarch64 | arm64) ARCH=armv8 ;;
  armv7l) ARCH=armv7 ;;
  *)
    echo "install-zwo-sdk: unsupported arch '$(uname -m)'" >&2
    exit 1
    ;;
esac

SUDO=""
[ "$(id -u)" -ne 0 ] && SUDO="sudo"

echo "install-zwo-sdk: arch=$ARCH prefix=$PREFIX ref=$REF"
$SUDO install -d "$PREFIX/lib" "$PREFIX/include"

# Headers (also vendored in-tree for bindgen; installed here for completeness).
for h in ASICamera2.h EFW_filter.h EAF_focuser.h license.txt; do
  $SUDO curl -fsSL "$BASE/$h" -o "$PREFIX/include/$h"
done

# Shared libraries (INDI's `.bin` == ZWO's upstream `.so`). Install under the
# linker name so `-lASICamera2` / `-lEFWFilter` resolve.
$SUDO curl -fsSL "$BASE/$ARCH/libASICamera2.bin" -o "$PREFIX/lib/libASICamera2.so"
$SUDO curl -fsSL "$BASE/$ARCH/libEFWFilter.bin" -o "$PREFIX/lib/libEFWFilter.so"
# EAF focuser is linked once the focuser is implemented; install it too so it is
# ready:
$SUDO curl -fsSL "$BASE/$ARCH/libEAFFocuser.bin" -o "$PREFIX/lib/libEAFFocuser.so" || true

if command -v ldconfig >/dev/null 2>&1; then
  $SUDO ldconfig
fi

echo "install-zwo-sdk: done -> $PREFIX/lib/libASICamera2.so, libEFWFilter.so"
