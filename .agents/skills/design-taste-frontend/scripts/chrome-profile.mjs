import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/** Allocate an owned child; caller-supplied directories are never removed. */
export function createChromeProfile(requested) {
  const roots = [os.tmpdir(), path.join(process.cwd(), '.taste-skill')];
  const contains = (root, target) => {
    const relative = path.relative(root, target);
    return relative === '' || (!relative.startsWith(`..${path.sep}`) && relative !== '..' && !path.isAbsolute(relative));
  };
  let base = path.resolve(requested || os.tmpdir());
  if (!roots.some(root => contains(path.resolve(root), base))) base = os.tmpdir();
  fs.mkdirSync(base, { recursive: true });
  const realBase = fs.realpathSync(base);
  const allowed = roots.filter(root => fs.existsSync(root)).some(root => contains(fs.realpathSync(root), realBase));
  if (!allowed) base = os.tmpdir();
  const directory = fs.mkdtempSync(path.join(base, 'taste-icons-'));
  return { directory, cleanup: () => fs.rmSync(directory, { recursive: true, force: true }) };
}
