#!/usr/bin/env node
const { spawn } = require('child_process');
const path = require('path');
const os = require('os');
const fs = require('fs');

// Path where the binary should be
const platform = os.platform();
const binName = platform === 'win32' ? 'sieve.exe' : 'sieve';
const distPath = path.join(__dirname, '..', 'dist', binName);

// Check if binary exists
if (!fs.existsSync(distPath)) {
    console.error(`Sieve binary not found at ${distPath}.`);
    console.error('Please try reinstalling: npm install -g sieve-cli');
    process.exit(1);
}

// Execute
const child = spawn(distPath, process.argv.slice(2), {
  stdio: 'inherit',
  env: process.env
});

child.on('exit', (code) => {
  process.exit(code ?? 0);
});

child.on('error', (err) => {
    console.error('Failed to start sieve:', err);
    process.exit(1);
});
