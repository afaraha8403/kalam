#!/usr/bin/env node

import { execSync, spawnSync } from 'child_process';
import { fileURLToPath } from 'url';
import { dirname, resolve } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const projectRoot = resolve(__dirname, '..');

const isWindows = process.platform === 'win32';
const VITE_PORT = 5173;

console.log(`Killing dev processes for project: ${projectRoot}\n`);

function killPid(pid, label) {
  if (!pid || isNaN(parseInt(pid))) return;
  try {
    if (isWindows) execSync(`taskkill /F /PID ${pid}`, { stdio: 'pipe' });
    else execSync(`kill -9 ${pid}`, { stdio: 'pipe' });
    console.log(`Killed ${label || 'process'} ${pid}`);
  } catch (_) {}
}

try {
  if (isWindows) {
    // 1) Kill by port (Vite dev server)
    try {
      const out = execSync(`netstat -ano | findstr :${VITE_PORT}`, { encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'] });
      const pids = new Set();
      for (const line of out.split('\n')) {
        const m = line.trim().split(/\s+/);
        const pid = m[m.length - 1];
        if (pid && /^\d+$/.test(pid)) pids.add(pid);
      }
      pids.forEach((pid) => killPid(pid, 'port ' + VITE_PORT));
    } catch (_) {
      // No process on port
    }

    // 2) Kill any process whose command line contains this project path (node, cargo, kalam-voice, etc.)
    const ps = spawnSync(
      'powershell',
      [
        '-NoProfile',
        '-Command',
        `Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -and $_.CommandLine.Contains($env:KILL_PROJECT_ROOT) } | ForEach-Object { $_.ProcessId }`,
      ],
      { encoding: 'utf8', env: { ...process.env, KILL_PROJECT_ROOT: projectRoot } }
    );
    if (ps.stdout && ps.status === 0) {
      const selfPid = String(process.pid);
      const pids = ps.stdout.trim().split(/\r?\n/).filter((p) => p && p !== selfPid);
      for (const pid of pids) killPid(pid, 'project');
    }
  } else {
    // macOS/Linux: kill by port first
    try {
      const pids = execSync(`lsof -ti:${VITE_PORT}`, { encoding: 'utf8' }).trim().split(/\s+/).filter(Boolean);
      pids.forEach((pid) => killPid(pid, 'port ' + VITE_PORT));
    } catch (_) {}

    // Then processes with project path in command line
    try {
      const result = execSync(
        `ps -eo pid,args | grep -E '(node|tauri|cargo|vite|kalam)' | grep -v grep | grep "${projectRoot}"`,
        { encoding: 'utf8' }
      );
      for (const line of result.trim().split('\n')) {
        const pid = line.trim().split(/\s+/)[0];
        if (pid && pid !== String(process.pid)) killPid(pid, 'project');
      }
    } catch (_) {}
  }

  console.log('\nDone.');
} catch (err) {
  console.error('Error:', err.message);
  process.exit(1);
}
