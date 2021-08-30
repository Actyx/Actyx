const HtmlWebpackPlugin = require("html-webpack-plugin")
const path = require("path")

module.exports = {
  entry: {
    app: { import: "./src/entry.ts" },
  },
  mode: "production",
  module: {
    rules: [
      {
        test: /\.tsx?$/,
        use: "ts-loader",
        exclude: /node_modules/,
      },
      {
        test: /\.css$/i,
        include: path.resolve(__dirname, "src"),
        use: ["style-loader", "css-loader", "postcss-loader"],
      },
    ],
  },
  resolve: {
    extensions: [".tsx", ".ts", ".js"],
  },
  output: {
    filename: "[name].bundle.js",
    path: path.resolve(__dirname, "dist"),
  },
  plugins: [
    new HtmlWebpackPlugin({
      templateContent: `
     <html>
      <body>
        <div id="app"></div>
      </body>`,
    }),
  ],
}
