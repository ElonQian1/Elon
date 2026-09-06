#!/usr/bin/env node
'use strict'

const { main } = require('./esk-sui-testnet-publication-preflight/cli')

process.exitCode = main(process.argv.slice(2))
