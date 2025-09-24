#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');

// Get the path to the binary
const binaryName = process.platform === 'win32' ? 'cc-switch.exe' : 'cc-switch';
const binaryPath = path.join(__dirname, 'bin', binaryName);

// Forward all arguments to the binary
const args = process.argv.slice(2);

// Spawn the binary with all arguments
const child = spawn(binaryPath, args, {
  stdio: 'inherit',
  shell: process.platform === 'win32'
});

// Handle child process events
child.on('error', (err) => {
  console.error('Failed to start cc-switch:', err.message);
  console.error('\nTry running the installation again: npm install cc-switch');
  process.exit(1);
});

child.on('exit', (code) => {
  process.exit(code);
});