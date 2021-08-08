// eslint-disable-next-line no-undef
module.exports = {
  /**
   * This is the main entry point for your application, it's the first file
   * that runs in the main process.
   */
  entry: './src/node/main.ts',
  // Put your normal webpack config below here
  target: 'electron-main',
  node: {
    __dirname: false,
  },
  module: {
    // eslint-disable-next-line no-undef
    rules: [
      {
        test: /\.node$/,
        loader: 'node-loader',
      },
    ].concat(require('./webpack.rules')),
  },
  resolve: {
    extensions: ['.js', '.ts', '.jsx', '.tsx', '.css', '.json', '.png'],
  },
}
