import { spawn } from 'node:child_process';
import process from 'node:process';

const root = process.env.TEST_WEB_ROOT || 'dist';
const port = process.env.PORT || '8000';

const child = spawn(
  'npx',
  ['http-server', root, '-p', port, '-c-1', '--cors'],
  { stdio: 'inherit' }
);

child.on('exit', (code) => {
  process.exit(code ?? 0);
});
