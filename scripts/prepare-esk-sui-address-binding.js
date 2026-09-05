#!/usr/bin/env node
const { main } = require('./esk-sui-address-binding/cli')

main(process.argv.slice(2)).then(code => { process.exitCode = code }).catch(() => {
  console.error('ESK_SUI_ADDRESS_BINDING_ERROR=INTERNAL_ERROR')
  process.exitCode = 1
})
