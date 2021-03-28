/// <reference types="cypress" />

describe('Final deliverable', () => {
  it('works', () => {
    cy.visit('/')
    let msg = Math.random().toString(36).substring(7)

    cy.get('input').type(msg)
    cy.get('button').click()
    cy.get('pre').contains(msg)
  })
})
