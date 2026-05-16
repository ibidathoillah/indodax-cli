#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

// Determine the binary name based on platform
const binName = process.platform === 'win32' ? 'indodax-native.exe' : 'indodax-native';
const binPath = path.join(__dirname, binName);

// Check if the binary exists
if (!fs.existsSync(binPath)) {
    // Fallback: Check if it's named 'indodax' (old naming)
    const oldBinName = process.platform === 'win32' ? 'indodax.exe' : 'indodax';
    const oldBinPath = path.join(__dirname, oldBinName);
    
    if (fs.existsSync(oldBinPath)) {
        runBinary(oldBinPath);
    } else {
        console.error(`\x1b[31mError: Indodax native binary not found.\x1b[0m`);
        console.error(`Expected at: ${binPath}`);
        console.error(`\nThis usually happens if the post-install download failed.`);
        console.error(`You can try to:`);
        console.error(`1. Reinstall: npm install -g indodax-cli`);
        console.error(`2. Build from source: cargo install --path .`);
        process.exit(1);
    }
} else {
    runBinary(binPath);
}

function runBinary(path) {
    const child = spawn(path, process.argv.slice(2), {
        stdio: 'inherit'
    });

    child.on('exit', (code) => {
        process.exit(code || 0);
    });

    child.on('error', (err) => {
        console.error(`\x1b[31mError spawning binary:\x1b[0m ${err.message}`);
        process.exit(1);
    });
}
