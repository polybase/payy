const assert = require('node:assert/strict')
const test = require('node:test')

const { normalizeProvingField } = require('../dist/proving')

test('proving backend rejects overwide public input fields as typed error', () => {
  assert.throws(() => normalizeProvingField(`0x${'11'.repeat(33)}`), {
    code: 'proof_output_malformed'
  })
})
