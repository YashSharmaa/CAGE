// Test CAGE JavaScript SDK
const { CAGEClient } = require('./dist/index.js');

async function test() {
    console.log('Testing CAGE JavaScript SDK...');
    console.log('='.repeat(60));

    const client = new CAGEClient(
        'http://127.0.0.1:8080',
        'dev_jssdk'
    );

    try {
        // Test 1: Execute
        console.log('\n[1/4] Testing execute()...');
        const result = await client.execute({
            code: "print('JS SDK Test: 456')"
        });
        console.log(`✓ Execute passed (status: ${result.status}, output: ${result.stdout.trim()})`);

        // Test 2: Health
        console.log('\n[2/4] Testing health()...');
        const health = await client.health();
        console.log(`✓ Health passed (status: ${health.status}, version: ${health.version})`);

        // Test 3: List files
        console.log('\n[3/4] Testing listFiles()...');
        const files = await client.listFiles();
        console.log(`✓ List files passed (${files.length} files)`);

        // Test 4: Execute JavaScript
        console.log('\n[4/4] Testing JavaScript execution...');
        const jsResult = await client.execute({
            code: "console.log(999)",
            language: 'javascript'
        });
        console.log(`✓ JavaScript exec passed (output: ${jsResult.stdout.trim()})`);

        console.log('\n' + '='.repeat(60));
        console.log('✅ ALL JAVASCRIPT SDK TESTS PASSED!');
        console.log('='.repeat(60));

    } catch (error) {
        console.error('\n❌ Test failed:', error.message);
        process.exit(1);
    }
}

test();
