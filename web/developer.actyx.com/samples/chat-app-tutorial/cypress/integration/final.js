/// <reference types="cypress" />
describe('Final deliverable', function () {
    it('works', function () {
        cy.visit('/');
        var msg = Math.random().toString(36).substring(7);
        cy.get('input').type(msg);
        cy.get('button').click();
        cy.get('pre').contains(msg);
    });
});
//# sourceMappingURL=final.js.map