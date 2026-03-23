describe('Settings STT mode (browser + mocked dev bridge)', () => {
  it('changes transcription mode and persists via save_settings', () => {
    cy.visit('/')
    cy.get('button[title="Settings"]').click()
    cy.contains('button', 'Audio & Dictation').click()
    cy.contains('h3', 'Speech-to-Text Mode').scrollIntoView()
    cy.contains('span.setting-name', 'Mode').parents('.setting-row').find('select').select('Hybrid')
    cy.wait(600)
    cy.get('button[title="Settings"]').click()
    cy.contains('button', 'Audio & Dictation').click()
    cy.contains('h3', 'Speech-to-Text Mode').scrollIntoView()
    cy.contains('span.setting-name', 'Mode').parents('.setting-row').find('select').should('have.value', 'Hybrid')
  })
})
