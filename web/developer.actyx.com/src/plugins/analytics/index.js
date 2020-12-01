const path = require("path");

module.exports = function (context, options) {
  // ...
  return {
    name: "analytics",
    getThemePath() {
      return path.resolve(__dirname, "./theme");
    },
    //async loadContent() {
    //  /* ... */
    //  console.log(`loadContent`);
    //},
    //async contentLoaded({ content, actions }) {
    //  /* ... */
    //  console.log(`contentLoaded`);
    //},
    /* other lifecycle API */
  };
};
