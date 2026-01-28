const fs = require('fs');
const path = require('path');
const os = require('os');
const axios = require('axios');
const tar = require('tar');

// Map node platform/arch to Rust target
const PLATFORMS = {
    'win32-x64': 'x86_64-pc-windows-msvc',
    'linux-x64': 'x86_64-unknown-linux-gnu',
    'darwin-x64': 'x86_64-apple-darwin',
    'darwin-arm64': 'aarch64-apple-darwin'
};

const key = `${os.platform()}-${os.arch()}`;
const target = PLATFORMS[key];

if (!target) {
    console.warn(`Sieve: Unsupported platform ${key}. You may need to build from source.`);
    process.exit(0);
}

const distDir = path.join(__dirname, '..', 'dist');
if (!fs.existsSync(distDir)) {
    fs.mkdirSync(distDir, { recursive: true });
}

// NOTE: Update this to match your repository
const VERSION = '0.1.0'; 
const REPO = '09Catho/Sieve';
const DOWNLOAD_URL = `https://github.com/${REPO}/releases/download/v${VERSION}/sieve-${target}.tar.gz`;

async function download() {
    console.log(`Downloading Sieve binary from ${DOWNLOAD_URL}...`);
    try {
        const response = await axios({
            method: 'get',
            url: DOWNLOAD_URL,
            responseType: 'stream'
        });

        const extract = tar.x({
            cwd: distDir,
            strict: true
        });

        response.data.pipe(extract);

        return new Promise((resolve, reject) => {
            extract.on('finish', () => {
                console.log(`Successfully installed Sieve to ${distDir}`);
                resolve();
            });
            extract.on('error', (err) => {
                reject(err);
            });
        });
    } catch (error) {
        console.error(`Error downloading Sieve: ${error.message}`);
        if (error.response) {
            console.error(`Status: ${error.response.status}`);
        }
        console.error('You may need to build from source: cargo install --path .');
        process.exit(1);
    }
}

download();
