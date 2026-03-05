#!/usr/bin/env node

import { execSync } from 'child_process';
import { fileURLToPath } from 'url';
import { dirname, resolve } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const projectRoot = resolve(__dirname, '..');

const isWindows = process.platform === 'win32';

console.log(`Killing processes for project: ${projectRoot}`);

try {
  if (isWindows) {
    // Windows: Find and kill node processes running from this project
    try {
      // Get list of node processes with their command lines
      const result = execSync('wmic process where "name=\'node.exe\'" get ProcessId,CommandLine /format:csv', { encoding: 'utf8' });
      const lines = result.trim().split('\n').slice(1); // Skip header
      
      for (const line of lines) {
        if (line.includes(projectRoot)) {
          const parts = line.split(',');
          const pid = parts[parts.length - 1]?.trim();
          if (pid && !isNaN(parseInt(pid))) {
            console.log(`Killing node process ${pid}`);
            try {
              execSync(`taskkill /F /PID ${pid}`);
            } catch (e) {
              // Process might already be dead
            }
          }
        }
      }
    } catch (e) {
      // WMIC might fail, fallback to killing by name
      console.log('Using fallback method...');
    }

    // Kill common process names
    const processesToKill = ['node.exe', 'tauri.exe', 'cargo.exe', 'rustc.exe'];
    for (const proc of processesToKill) {
      try {
        execSync(`taskkill /F /IM ${proc} 2>nul`);
        console.log(`Killed ${proc}`);
      } catch (e) {
        // Process not running
      }
    }
  } else {
    // macOS/Linux
    // Kill processes that have this project path in their command line
    try {
      const result = execSync(`ps aux | grep -E '(node|tauri|cargo)' | grep -v grep | grep "${projectRoot}"`, { encoding: 'utf8' });
      const lines = result.trim().split('\n');
      
      for (const line of lines) {
        const parts = line.trim().split(/\s+/);
        const pid = parts[1];
        if (pid && !isNaN(parseInt(pid))) {
          console.log(`Killing process ${pid}: ${parts.slice(10).join(' ')}`);
          try {
            execSync(`kill -9 ${pid}`);
          } catch (e) {
            // Process might already be dead
          }
        }
      }
    } catch (e) {
      // No matching processes
    }

    // Also try killing by port (Vite default 5173, Tauri default 1420)
    try {
      execSync('lsof -ti:5173 | xargs kill -9 2>/dev/null');
      console.log('Killed process on port 5173');
    } catch (e) {}
    
    try {
      execSync('lsof -ti:1420 | xargs kill -9 2>/dev/null');
      console.log('Killed process on port 1420');
    } catch (e) {}
  }

  console.log('Done!');
} catch (error) {
  console.error('Error killing processes:', error.message);
  process.exit(1);
}
