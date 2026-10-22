const { execSync } = require('child_process');

console.log('Running secret scan over full git history...');
const log = execSync('git log -p', { maxBuffer: 100 * 1024 * 1024 }).toString('utf-8');

const patterns = [
  { name: 'AWS Access Key ID', regex: /AKIA[0-9A-Z]{16}/g },
  { name: 'AWS Secret Key pattern', regex: /(aws_secret_access_key|aws_secret_key)\s*[:=]\s*['\"][A-Za-z0-9\/+=]{40}['\"]/gi },
  { name: 'Groq API Key', regex: /gsk_[a-zA-Z0-9]{40,}/g },
  { name: 'OpenAI API Key', regex: /sk-[a-zA-Z0-9]{32,}/g },
  { name: 'Anthropic API Key', regex: /sk-ant-[a-zA-Z0-9_\-]{32,}/g },
  { name: 'Private Key Block', regex: /-----BEGIN (RSA|EC|OPENSSH|DSA|PGP|PRIVATE) KEY/g },
  { name: 'GitHub Token', regex: /(ghp_[0-9a-zA-Z]{36}|github_pat_[0-9a-zA-Z_]{82})/g },
  { name: 'Slack Token', regex: /xox[baprs]-[0-9a-zA-Z]{10,48}/g },
];

let foundSecrets = [];
const lines = log.split('\n');
let currentCommit = '';

for (const line of lines) {
  if (line.startsWith('commit ')) {
    currentCommit = line.trim();
  }
  // Only scan added lines in diffs (starts with + but not +++)
  if (line.startsWith('+') && !line.startsWith('+++')) {
    for (const pat of patterns) {
      const matches = line.match(pat.regex);
      if (matches) {
        // filter out known safe documentation or test placeholders
        const filtered = matches.filter(m =>
          !m.includes('EXAMPLE') &&
          !m.includes('placeholder') &&
          !m.includes('your-groq-api-key') &&
          !m.includes('test_token')
        );
        if (filtered.length > 0) {
          foundSecrets.push({ commit: currentCommit, type: pat.name, match: filtered, line: line.trim() });
        }
      }
    }
  }
}

console.log(`Scanned ${lines.length} lines across git history.`);
if (foundSecrets.length === 0) {
  console.log('[PASS] Secret scan passed: 0 secrets detected in git history.');
  process.exit(0);
} else {
  console.error('[FAIL] Secret scan failed: Secrets found in git history:');
  console.error(JSON.stringify(foundSecrets, null, 2));
  process.exit(1);
}
