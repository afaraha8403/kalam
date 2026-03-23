import { handleDevBridgeInvoke, resetMockConfig } from './dev-bridge-handlers'

// Default profile for specs that forget to reset (main app shell).
beforeEach(() => {
  resetMockConfig('main')
  cy.intercept('POST', 'http://127.0.0.1:1430/api/invoke', (req) => {
    const raw = req.body
    const body = typeof raw === 'string' ? (JSON.parse(raw) as Parameters<typeof handleDevBridgeInvoke>[0]) : raw
    const out = handleDevBridgeInvoke(body)
    // Cypress 15 rejects req.reply(null); mirror JSON bodies the real bridge returns.
    req.reply({
      statusCode: 200,
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(out === undefined ? null : out),
    })
  })
})
