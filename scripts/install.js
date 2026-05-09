const fs = require('fs');
const path = require('path');
const https = require('https');
const os = require('os');

const VERSION = '0.1.2';
const REPO = 'ibidathoillah/indodax-cli';

function getBinaryUrl() {
    const platform = os.platform();
    const arch = os.arch();

    let osName = '';
    let archName = '';

    if (platform === 'linux') {
        osName = 'linux';
    } else if (platform === 'darwin') {
        osName = 'macos';
    } else {
        console.error(`Unsupported platform: ${platform}`);
        process.exit(1);
    }

    if (arch === 'x64') {
        archName = 'x86_64';
    } else if (arch === 'arm64') {
        archName = 'aarch64';
    } else {
        console.error(`Unsupported architecture: ${arch}`);
        process.exit(1);
    }

    return `https://github.com/${REPO}/releases/download/v${VERSION}/indodax-${osName}-${archName}`;
}

const binDir = path.join(__dirname, '..', 'bin');
const binPath = path.join(binDir, 'indodax');

if (!fs.existsSync(binDir)) {
    fs.mkdirSync(binDir);
}

const url = getBinaryUrl();
console.log(`Downloading indodax-cli binary from ${url}...`);

const file = fs.createWriteStream(binPath);
https.get(url, (response) => {
    if (response.statusCode === 302 || response.statusCode === 301) {
        https.get(response.headers.location, (followResponse) => {
            followResponse.pipe(file);
        });
    } else {
        response.pipe(file);
    }

    file.on('finish', () => {
        file.close();
        fs.chmodSync(binPath, 0o755);
        console.log('indodax-cli installed successfully.');
    });
}).on('error', (err) => {
    fs.unlink(binPath, () => {});
    console.error(`Error downloading binary: ${err.message}`);
    process.exit(1);
});
