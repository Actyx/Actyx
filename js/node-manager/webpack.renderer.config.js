// eslint-disable-next-line
const rules = require('./webpack.rules')
// eslint-disable-next-line
const plugins = require('./webpack.plugins')
const HtmlWebpackPlugin = require('html-webpack-plugin')

// eslint-disable-next-line no-undef
module.exports = {
  module: {
    rules,
  },
  plugins: plugins.concat([
    new HtmlWebpackPlugin({
      title: 'Actyx Node Manager',
    }),
  ]),
  resolve: {
    extensions: ['.js', '.ts', '.jsx', '.tsx', '.css', '.png'],
  },
  output: {},
}
