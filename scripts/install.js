const fs = require('fs');
const path = require('path');
const https = require('https');
const os = require('os');
const { spawnSync } = require('child_process');

// Get version from package.json
const pkg = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'package.json'), 'utf8'));
const VERSION = pkg.version;
const REPO = 'ibidathoillah/indodax-cli';

const binDir = path.join(__dirname, '..', 'bin');
const platform = os.platform();
const arch = os.arch();
const binName = platform === 'win32' ? 'indodax-native.exe' : 'indodax-native';
const binPath = path.join(binDir, binName);

if (!fs.existsSync(binDir)) {
    fs.mkdirSync(binDir, { recursive: true });
}

function getReleaseBinaryUrl() {
    let osName = '';
    let archName = '';

    if (platform === 'linux') {
        osName = 'linux';
    } else if (platform === 'darwin') {
        osName = 'macos';
    } else if (platform === 'win32') {
        osName = 'windows';
    } else {
        console.error(`Unsupported platform for release download: ${platform}`);
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

    const ext = platform === 'win32' ? '.exe' : '';
    return `https://github.com/${REPO}/releases/download/v${VERSION}/indodax-${osName}-${archName}${ext}`;
}

function getSourceBinaryPath() {
    return path.join(__dirname, '..', 'target', 'release', platform === 'win32' ? 'indodax.exe' : 'indodax');
}

function buildFromSource() {
    console.log('Building indodax-cli from source for Android/Termux...');

    const result = spawnSync('cargo', ['build', '--release'], {
        cwd: path.join(__dirname, '..'),
        stdio: 'inherit',
    });

    if (result.error) {
        console.error(`\x1b[31mError running cargo:\x1b[0m ${result.error.message}`);
        process.exit(1);
    }

    if (result.status !== 0) {
        process.exit(result.status || 1);
    }

    const sourceBinary = getSourceBinaryPath();
    if (!fs.existsSync(sourceBinary)) {
        console.error(`\x1b[31mError:\x1b[0m built binary not found at ${sourceBinary}`);
        process.exit(1);
    }

    fs.copyFileSync(sourceBinary, binPath);
    fs.chmodSync(binPath, 0o755);
    console.log('\x1b[32mindodax-cli binary installed successfully.\x1b[0m');
}

function download(url, dest) {
    console.log(`Downloading indodax-cli binary from ${url}...`);

    const file = fs.createWriteStream(dest);

    https.get(url, (response) => {
        if (response.statusCode === 301 || response.statusCode === 302) {
            download(response.headers.location, dest);
            return;
        }

        if (response.statusCode !== 200) {
            console.warn(`\x1b[33mWarning:\x1b[0m Server returned status code ${response.statusCode}`);
            if (response.statusCode === 404) {
                console.warn(`Binary for version v${VERSION} not found on GitHub releases.`);
                console.warn(`The command 'indodax' will still be registered, but you may need to build it manually.`);
            }
            fs.unlink(dest, () => { });
            // Exit with 0 to allow the npm installation to complete
            process.exit(0);
        }

        response.pipe(file);

        file.on('finish', () => {
            file.close();
            fs.chmodSync(dest, 0o755);
            console.log('\x1b[32mindodax-cli binary installed successfully.\x1b[0m');
        });
    }).on('error', (err) => {
        fs.unlink(dest, () => { });
        console.error(`\x1b[31mError downloading binary:\x1b[0m ${err.message}`);
        process.exit(1);
    });
}

if (platform === 'android') {
    buildFromSource();
} else {
    const url = getReleaseBinaryUrl();
    download(url, binPath);
}
