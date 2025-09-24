#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const https = require('https');
const { execSync } = require('child_process');

const packageJson = require('./package.json');
const version = packageJson.version;

// Platform and architecture mapping
const platformMap = {
  'darwin': {
    'x64': 'x86_64-apple-darwin',
    'arm64': 'aarch64-apple-darwin'
  },
  'linux': {
    'x64': 'x86_64-unknown-linux-musl',
    'arm64': 'aarch64-unknown-linux-musl'
  },
  'win32': {
    'x64': 'x86_64-pc-windows-gnu',
    'arm64': 'aarch64-pc-windows-gnu'
  }
};

function getPlatformTarget() {
  const platform = process.platform;
  const arch = process.arch;

  if (!platformMap[platform]) {
    throw new Error(`Unsupported platform: ${platform}`);
  }

  if (!platformMap[platform][arch]) {
    throw new Error(`Unsupported architecture: ${arch} on ${platform}`);
  }

  return platformMap[platform][arch];
}

function getBinaryName() {
  return process.platform === 'win32' ? 'cc-switch.exe' : 'cc-switch';
}

function getDownloadUrl(target) {
  return `https://github.com/Linuxdazhao/cc_auto_switch/releases/download/v${version}/cc-switch-${target}.tar.gz`;
}

function downloadFile(url, dest) {
  return new Promise((resolve, reject) => {
    console.log(`Downloading ${url}...`);

    const file = fs.createWriteStream(dest);

    https.get(url, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        // Handle redirects
        return downloadFile(response.headers.location, dest).then(resolve).catch(reject);
      }

      if (response.statusCode !== 200) {
        reject(new Error(`Download failed with status ${response.statusCode}`));
        return;
      }

      response.pipe(file);

      file.on('finish', () => {
        file.close();
        resolve();
      });

      file.on('error', (err) => {
        fs.unlink(dest, () => {}); // Delete the file on error
        reject(err);
      });
    }).on('error', (err) => {
      reject(err);
    });
  });
}

function extractTarGz(tarPath, extractDir) {
  try {
    console.log(`Extracting ${tarPath}...`);

    // Ensure extract directory exists
    if (!fs.existsSync(extractDir)) {
      fs.mkdirSync(extractDir, { recursive: true });
    }

    // Use node's built-in tar extraction if available, otherwise use system tar
    if (process.platform === 'win32') {
      // On Windows, try to use PowerShell or 7zip
      try {
        execSync(`powershell -command "Expand-Archive -Path '${tarPath}' -DestinationPath '${extractDir}'"`, { stdio: 'inherit' });
      } catch (e) {
        // Fallback to tar if available
        execSync(`tar -xzf "${tarPath}" -C "${extractDir}"`, { stdio: 'inherit' });
      }
    } else {
      // On Unix-like systems, use tar
      execSync(`tar -xzf "${tarPath}" -C "${extractDir}"`, { stdio: 'inherit' });
    }

    console.log('Extraction completed');
  } catch (error) {
    throw new Error(`Failed to extract archive: ${error.message}`);
  }
}

async function install() {
  try {
    const target = getPlatformTarget();
    const binaryName = getBinaryName();
    const downloadUrl = getDownloadUrl(target);

    console.log(`Installing cc-switch v${version} for ${target}...`);

    // Create bin directory
    const binDir = path.join(__dirname, 'bin');
    if (!fs.existsSync(binDir)) {
      fs.mkdirSync(binDir, { recursive: true });
    }

    // Download archive
    const archivePath = path.join(__dirname, `cc-switch-${target}.tar.gz`);
    await downloadFile(downloadUrl, archivePath);

    // Extract archive
    const extractDir = path.join(__dirname, 'temp');
    extractTarGz(archivePath, extractDir);

    // Move binary to bin directory
    const sourceBinary = path.join(extractDir, binaryName);
    const destBinary = path.join(binDir, 'cc-switch' + (process.platform === 'win32' ? '.exe' : ''));

    if (!fs.existsSync(sourceBinary)) {
      throw new Error(`Binary not found at ${sourceBinary}`);
    }

    fs.copyFileSync(sourceBinary, destBinary);

    // Make binary executable on Unix-like systems
    if (process.platform !== 'win32') {
      fs.chmodSync(destBinary, '755');
    }

    // Clean up
    fs.unlinkSync(archivePath);
    fs.rmSync(extractDir, { recursive: true, force: true });

    console.log(`✅ cc-switch v${version} installed successfully!`);
    console.log(`📍 Binary location: ${destBinary}`);
    console.log(`🚀 You can now use: npx cc-switch or add to PATH for global access`);

  } catch (error) {
    console.error(`❌ Installation failed: ${error.message}`);
    console.error(`\n📋 Troubleshooting:`);
    console.error(`   • Ensure you have internet connectivity`);
    console.error(`   • Check if the release v${version} exists on GitHub`);
    console.error(`   • Try installing directly from GitHub: https://github.com/Linuxdazhao/cc_auto_switch/releases`);
    process.exit(1);
  }
}

// Run installation
if (require.main === module) {
  install();
}

module.exports = { install };