const dns = require('node:dns')
const dgram = require('node:dgram')
const http = require('node:http')
const http2 = require('node:http2')
const https = require('node:https')
const net = require('node:net')
const tls = require('node:tls')

function blocked() { throw new Error('NETWORK_ATTEMPT_BLOCKED') }

dns.lookup = blocked
dns.resolve = blocked
dns.promises.lookup = async () => blocked()
dns.promises.resolve = async () => blocked()
dgram.createSocket = blocked
http.request = blocked
http.get = blocked
http2.connect = blocked
https.request = blocked
https.get = blocked
net.connect = blocked
net.createConnection = blocked
net.Socket.prototype.connect = blocked
tls.connect = blocked
globalThis.fetch = blocked
globalThis.WebSocket = class NetworkBlockedWebSocket { constructor() { blocked() } }
