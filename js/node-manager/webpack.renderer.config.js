// eslint-disable-next-line
const rules = require('./webpack.rules')
const path = require('path')
// eslint-disable-next-line
const plugins = require('./webpack.plugins')
const HtmlWebpackPlugin = require('html-webpack-plugin')

// eslint-disable-next-line no-undef
module.exports = {
  module: {
    rules,
  },
  externals: {
    react: 'react',
  },
  plugins: plugins.concat([
    new HtmlWebpackPlugin({
      title: 'Actyx Node Manager',
    }),
  ]),
  resolve: {
    extensions: ['.js', '.ts', '.jsx', '.tsx', '.css', '.png'],
    //alias: {
    //  react: path.resolve('./node_modules/react'),
    //},
    //alias: {
    //  react: path.resolve(path.join(__dirname, './node_modules/react')),
    //  'react-dom': path.resolve(path.join(__dirname, './node_modules/react-dom')),
    //},
  },
  output: {},
}
