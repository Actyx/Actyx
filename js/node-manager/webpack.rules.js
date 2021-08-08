// eslint-disable-next-line no-undef
module.exports = [
  {
    test: /\.css$/,
    use: ['css-loader', 'postcss-loader'],
  },
  {
    test: /\.ttf$|\.png$/,
    use: ['file-loader'],
  },
]
