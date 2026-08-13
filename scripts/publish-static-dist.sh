#!/usr/bin/env bash

activate_static_dist() {
  local staging_dir="$1" remote_dir="$2" release_sha="$3" expected_release_sha="${4:-}"
  mkdir -p "$remote_dir/assets"
  exec 9>"$remote_dir/.pc-static-publish.lock"
  flock -w 60 9
  if [ -n "$expected_release_sha" ]; then
    local current_release_sha="__missing__"
    [ ! -f "$remote_dir/release-sha.txt" ] || current_release_sha="$(tr -d '[:space:]' < "$remote_dir/release-sha.txt")"
    if [ "$current_release_sha" != "$expected_release_sha" ]; then
      echo "static release changed: expected=$expected_release_sha actual=$current_release_sha" >&2
      return 42
    fi
  fi
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
  local staging_dir="$1" remote_dir="$2" release_sha="$3" expected_release_sha="${4:-}"
  # SSH_OPTS and SERVER are supplied by the release entrypoint.
  # shellcheck disable=SC2086
  ssh $SSH_OPTS "$SERVER" bash -s -- "$staging_dir" "$remote_dir" "$release_sha" "$expected_release_sha" <<'REMOTE_STATIC_PUBLISH'
set -eu
staging_dir="$1"; remote_dir="$2"; release_sha="$3"; expected_release_sha="$4"
mkdir -p "$remote_dir"
exec 9>"$remote_dir/.pc-static-publish.lock"
flock -w 60 9
if [ -n "$expected_release_sha" ]; then
  current_release_sha="__missing__"
  [ ! -f "$remote_dir/release-sha.txt" ] || current_release_sha="$(tr -d '[:space:]' < "$remote_dir/release-sha.txt")"
  if [ "$current_release_sha" != "$expected_release_sha" ]; then
    echo "static release changed: expected=$expected_release_sha actual=$current_release_sha" >&2
    exit 42
  fi
fi
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

upload_static_dist() {
  local local_dir="$1" remote_dir="$2" label="$3"
  local required="${4:-0}" expected_release_sha="${5:-}"
  local staging_dir="${remote_dir}-staging-$SHA"

  if [ -z "$local_dir" ] || [ ! -f "$local_dir/index.html" ]; then
    echo -e "${YELLOW}3.5⃣  ⚠️  $label 不存在，跳过上传${NC}"
    [ "$required" -eq 1 ] && return 1
    return 0
  fi

  echo -e "${YELLOW}3.5⃣  上传 $label 到 $remote_dir ...${NC}"
  if [ "$LOCAL_DEPLOY" -eq 1 ]; then
    rm -rf "$staging_dir"; mkdir -p "$staging_dir"
    cp -a "$local_dir"/. "$staging_dir"/
    activate_static_dist "$staging_dir" "$remote_dir" "$SHA" "$expected_release_sha"
  else
    # shellcheck disable=SC2086
    ssh $SSH_OPTS "$SERVER" "mkdir -p '$staging_dir'" || { [ "$required" -ne 1 ] || return 1; return 0; }
    if ! scp $SSH_OPTS -r "$local_dir/." "${SERVER}:${staging_dir}"; then
      # shellcheck disable=SC2086
      ssh $SSH_OPTS "$SERVER" "rm -rf '$staging_dir'" 2>/dev/null || true
      [ "$required" -ne 1 ] || return 1; return 0
    fi
    if ! activate_remote_static_dist "$staging_dir" "$remote_dir" "$SHA" "$expected_release_sha"; then
      # shellcheck disable=SC2086
      ssh $SSH_OPTS "$SERVER" "rm -rf '$staging_dir'" 2>/dev/null || true
      [ "$required" -ne 1 ] || return 1; return 0
    fi
  fi
  echo -e "${GREEN}   ✅ $label 原子入口发布完成（旧 hash 保留宽限期）→ $remote_dir${NC}"
}
