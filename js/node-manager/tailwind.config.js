module.exports = {
  purge: [],
  darkMode: 'class', // or 'media' or 'class'
  theme: {
    extend: {},
    gridTemplateColumns: {
      nodes: 'repeat( auto-fit, 14rem)',
    },
    gridTemplateRows: {
      nodes: 'repeat( auto-fit, auto)',
    },
  },
  variants: {
    extend: {},
  },
  plugins: [require('@tailwindcss/forms')],
}
