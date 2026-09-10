import { execFileSync, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

export function git(root, args) {
  return execFileSync('git', ['-C', root, ...args], {
    encoding: 'utf8', windowsHide: true, maxBuffer: 64 * 1024 * 1024, stdio: ['ignore', 'pipe', 'pipe'],
  });
}

export function repository(root) {
  return git(path.resolve(root), ['rev-parse', '--show-toplevel']).trim();
}

export function resolveRef(root, ref) {
  return git(root, ['rev-parse', '--verify', '--end-of-options', ref + '^{commit}']).trim();
}

// Keep filenames NUL-delimited and separate index/tree contents from working files.
export function snapshot(root, mode = 'worktree', ref) {
  const entries = new Map();
  if (mode === 'worktree') {
    const files = git(root, ['ls-files', '--cached', '--others', '--exclude-standard', '-z']).split('\0').filter(Boolean);
    for (const file of files) {
      const full = path.join(root, file);
      try {
        const segments = file.split('/');
        let parent = root;
        let linked = false;
        for (const segment of segments) {
          parent = path.join(parent, segment);
          if (fs.lstatSync(parent).isSymbolicLink()) { linked = true; break; }
        }
        if (!linked && fs.lstatSync(full).isFile()) entries.set(file, { full });
      } catch (error) {
        if (error.code !== 'ENOENT' && error.code !== 'ENOTDIR') throw error;
      }
    }
  } else {
    const args = mode === 'index' ? ['ls-files', '--stage', '-z'] : ['ls-tree', '-r', '-z', ref];
    for (const record of git(root, args).split('\0').filter(Boolean)) {
      const tab = record.indexOf('\t');
      const [permissions, second, third] = record.slice(0, tab).split(' ');
      if (mode === 'index' && third !== '0') throw new Error('Resolve merge conflicts before running the guard');
      if (!['100644', '100755'].includes(permissions)) continue;
      entries.set(record.slice(tab + 1), { oid: mode === 'index' ? second : third });
    }
  }
  const cache = new Map();
  function readMany(files) {
    const requested = files.filter(file => entries.has(file) && !cache.has(file));
    if (mode === 'worktree') {
      for (const file of requested) cache.set(file, fs.readFileSync(entries.get(file).full));
    } else if (requested.length) {
      const child = spawnSync('git', ['-C', root, 'cat-file', '--batch'], {
        input: requested.map(file => entries.get(file).oid).join('\n') + '\n',
        windowsHide: true, maxBuffer: 256 * 1024 * 1024,
      });
      if (child.error) throw child.error;
      if (child.status !== 0) throw new Error('git cat-file failed: ' + child.stderr?.toString());
      let offset = 0;
      for (const file of requested) {
        const end = child.stdout.indexOf(10, offset);
        const header = child.stdout.subarray(offset, end).toString('utf8').split(' ');
        const size = Number(header[2]);
        if (end < 0 || header[1] !== 'blob' || !Number.isSafeInteger(size)) throw new Error('Invalid Git blob response');
        cache.set(file, child.stdout.subarray(end + 1, end + 1 + size));
        offset = end + size + 2;
      }
    }
    return cache;
  }
  function text(file) {
    if (!entries.has(file)) return null;
    return new TextDecoder('utf-8', { fatal: true }).decode(readMany([file]).get(file));
  }
  return { entries, readMany, text };
}
