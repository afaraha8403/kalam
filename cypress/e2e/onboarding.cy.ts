import { resetMockConfig } from '../support/dev-bridge-handlers'

describe('Onboarding (browser + mocked dev bridge)', () => {
  beforeEach(() => {
    resetMockConfig('onboarding')
  })

  it('walks through all onboarding steps', () => {
    cy.visit('/?e2eOnboarding=1')
    
    // Step 1: Welcome
    cy.contains('h1', 'Welcome to Kalam').should('be.visible')
    cy.get('.actions .btn-next').click()
    
    // Step 2: Email & terms
    cy.contains('h1', 'Email & terms').should('be.visible')
    cy.get('#onboarding-email').type('e2e@example.com')
    cy.get('.step-account input[type=checkbox]').first().check()
    cy.get('.actions .btn-next').click()
    
    // Step 3: Access & microphone
    cy.contains('h1', 'Access & microphone').should('be.visible')
    cy.contains('Microphone').should('be.visible')
    cy.get('.actions .btn-next').should('not.be.disabled')
    cy.get('.actions .btn-next').click()
    
    // Step 4: Engine
    cy.contains('h1', 'Speech engine').should('exist')
    cy.get('.actions .btn-next').scrollIntoView().click()
    
    // Step 5: Shortcuts
    cy.contains('h1', 'Shortcuts & languages').should('exist')
    cy.get('.actions .btn-next').scrollIntoView().click()
    
    // Step 6: Try it
    cy.contains('h1', 'Try dictation once').should('exist')
    cy.get('.actions .btn-finish').scrollIntoView().click()
    
    // Should be on the main app now
    cy.get('.sidebar').should('be.visible')
  })
})
