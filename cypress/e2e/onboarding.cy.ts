import { resetMockConfig } from '../support/dev-bridge-handlers'

describe('Onboarding (browser + mocked dev bridge)', () => {
  beforeEach(() => {
    resetMockConfig('onboarding')
  })

  it('walks welcome and account steps', () => {
    cy.visit('/?e2eOnboarding=1')
    cy.contains('h1', 'Welcome to Kalam').should('be.visible')
    cy.get('.actions .btn-next').click()
    cy.contains('h1', 'Create your account').should('be.visible')
    cy.get('#onboarding-email').type('e2e@example.com')
    cy.get('.step-account input[type=checkbox]').first().check()
    cy.get('.actions .btn-next').click()
    cy.contains('h1', 'Permissions').should('be.visible')
  })
})
