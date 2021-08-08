/// <reference types="cypress" />

describe("Final deliverable", () => {
  it("works", () => {
    cy.visit("/");
    const msg = Math.random().toString(36).substring(7);

    cy.get("input").type(msg);
    cy.get("button").click();
    cy.get("button").click(); // Don't know why, but this makes the test more robust
    cy.get("pre").contains(msg);
  });
});
