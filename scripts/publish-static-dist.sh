#!/usr/bin/env bash

activate_static_dist() {
  local staging_dir="$1" remote_dir="$2" release_sha="$3"
  mkdir -p "$remote_dir/assets"
  if [ -f "$remote_dir/index.html" ]; then
    { grep -oE 'assets/[A-Za-z0-9._/-]+' "$remote_dir/index.html" || true; } |
      sed 's#^assets/##' | while IFS= read -r asset; do
        [ ! -f "$remote_dir/assets/$asset" ] || touch "$remote_dir/assets/$asset"
      done
  fi
  [ ! -d "$staging_dir/assets" ] || cp -a "$staging_dir/assets"/. "$remote_dir/assets"/
  local item base
  for item in "$staging_dir"/*; do
    [ -e "$item" ] || continue
    base="$(basename "$item")"
    [ "$base" = assets ] && continue
    [ "$base" = index.html ] && continue
    if [ -f "$item" ]; then
      cp "$item" "$remote_dir/.publish-new-$base"
      mv -f "$remote_dir/.publish-new-$base" "$remote_dir/$base"
    fi
  done
  cp "$staging_dir/index.html" "$remote_dir/.publish-new-index-$release_sha"
  mv -f "$remote_dir/.publish-new-index-$release_sha" "$remote_dir/index.html"
  if [ ! -f "$remote_dir/.atomic-static-retention" ]; then
    touch "$remote_dir/.atomic-static-retention"
  elif find "$remote_dir/.atomic-static-retention" -mtime +14 -print -quit | grep -q .; then
    find "$remote_dir/assets" -type f -mtime +14 -delete
    touch "$remote_dir/.atomic-static-retention"
  fi
  rm -rf "$staging_dir"
}

activate_remote_static_dist() {
  local staging_dir="$1" remote_dir="$2" release_sha="$3"
  # SSH_OPTS and SERVER are supplied by the release entrypoint.
  # shellcheck disable=SC2086
  ssh $SSH_OPTS "$SERVER" bash -s -- "$staging_dir" "$remote_dir" "$release_sha" <<'REMOTE_STATIC_PUBLISH'
set -eu
staging_dir="$1"; remote_dir="$2"; release_sha="$3"
mkdir -p "$remote_dir/assets"
if [ -f "$remote_dir/index.html" ]; then
  { grep -oE 'assets/[A-Za-z0-9._/-]+' "$remote_dir/index.html" || true; } |
    sed 's#^assets/##' | while IFS= read -r asset; do
      [ ! -f "$remote_dir/assets/$asset" ] || touch "$remote_dir/assets/$asset"
    done
fi
[ ! -d "$staging_dir/assets" ] || cp -a "$staging_dir/assets"/. "$remote_dir/assets"/
for item in "$staging_dir"/*; do
  [ -e "$item" ] || continue
  base="$(basename "$item")"
  [ "$base" = assets ] && continue
  [ "$base" = index.html ] && continue
  if [ -f "$item" ]; then
    cp "$item" "$remote_dir/.publish-new-$base"
    mv -f "$remote_dir/.publish-new-$base" "$remote_dir/$base"
  fi
done
cp "$staging_dir/index.html" "$remote_dir/.publish-new-index-$release_sha"
mv -f "$remote_dir/.publish-new-index-$release_sha" "$remote_dir/index.html"
if [ ! -f "$remote_dir/.atomic-static-retention" ]; then
  touch "$remote_dir/.atomic-static-retention"
elif find "$remote_dir/.atomic-static-retention" -mtime +14 -print -quit | grep -q .; then
  find "$remote_dir/assets" -type f -mtime +14 -delete
  touch "$remote_dir/.atomic-static-retention"
fi
rm -rf "$staging_dir"
REMOTE_STATIC_PUBLISH
}
