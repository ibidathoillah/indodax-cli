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

    if (platform === 'linux' || platform === 'android') {
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
    return path.join(__dirname, '..', 'target', 'release', platform === 'win32' ? 'indodax-cli.exe' : 'indodax-cli');
}

function buildFromSource() {
    console.log('Building indodax-cli from source...');

    const result = spawnSync('cargo', ['build', '--release', '--all-features'], {
        cwd: path.join(__dirname, '..'),
        stdio: 'inherit',
    });

    if (result.error) {
        console.error(`\x1b[31mError running cargo:\x1b[0m ${result.error.message}`);
        if (platform === 'android' && result.error.code === 'ENOENT') {
            console.error(`\x1b[33mTip:\x1b[0m You need to install Rust and Cargo to build from source on Termux.`);
            console.error(`Run: \x1b[32mpkg install rust\x1b[0m`);
        }
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
    const tempDest = `${dest}.download`;

    https.get(url, (response) => {
        if (response.statusCode === 301 || response.statusCode === 302) {
            download(response.headers.location, dest);
            return;
        }

        if (response.statusCode !== 200) {
            console.warn(`\x1b[33mWarning:\x1b[0m Server returned status code ${response.statusCode}`);
            if (platform === 'android') {
                console.warn('Falling back to a local Termux build from source.');
                if (fs.existsSync(tempDest)) fs.unlinkSync(tempDest);
                buildFromSource();
                return;
            }
            if (response.statusCode === 404) {
                console.warn(`Binary for version v${VERSION} not found on GitHub releases.`);
                console.warn(`The command 'indodax' will still be registered, but you may need to build it manually.`);
            }
            if (fs.existsSync(tempDest)) fs.unlinkSync(tempDest);
            // Exit with 0 to allow the npm installation to complete
            process.exit(0);
        }

        const file = fs.createWriteStream(tempDest);
        response.pipe(file);

        file.on('finish', () => {
            file.close();
            fs.renameSync(tempDest, dest);
            fs.chmodSync(dest, 0o755);

            // On Android, check if the downloaded binary actually works
            if (platform === 'android') {
                const check = spawnSync(dest, ['--version']);
                if (check.status !== 0 || check.error) {
                    console.warn('\x1b[33mWarning:\x1b[0m Downloaded binary is not compatible with this Android environment.');
                    console.warn('Falling back to a local Termux build from source.');
                    buildFromSource();
                    return;
                }
            }

            console.log('\x1b[32mindodax-cli binary installed successfully.\x1b[0m');
        });
    }).on('error', (err) => {
        if (platform === 'android') {
            console.warn(`\x1b[33mWarning:\x1b[0m Download failed, falling back to a local Termux build from source.`);
            if (fs.existsSync(tempDest)) fs.unlinkSync(tempDest);
            buildFromSource();
            return;
        }
        if (fs.existsSync(tempDest)) fs.unlinkSync(tempDest);
        console.error(`\x1b[31mError downloading binary:\x1b[0m ${err.message}`);
        process.exit(1);
    });
}

download(getReleaseBinaryUrl(), binPath);
