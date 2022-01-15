const CleanWebpackPlugin = require('clean-webpack-plugin').CleanWebpackPlugin;
const HtmlWebPackPlugin = require("html-webpack-plugin");
const TsconfigPathsPlugin = require('tsconfig-paths-webpack-plugin');
const path = require("path");

const htmlPlugin = new HtmlWebPackPlugin({
  template: "./src/index.html",
  filename: "./index.html",
  inject: false
});

module.exports = {
  mode: "none",
  devtool: "inline-source-map",
	entry: "./src/index.tsx", // Point to main file
	output: {
		filename: "index.js",
		path: path.resolve(__dirname, "dist")
	},
	resolve: {
    extensions: ['.js', '.jsx', '.ts', '.tsx'],
    plugins: [new TsconfigPathsPlugin()]
	},
	performance: {
		hints: false
	},
	module: {
		rules: [
			{
				test: /\.scss$/,
				use: [
					"style-loader", 						// creates style nodes from JS strings
					"css-loader", 							// translates CSS into CommonJS
					"sass-loader" 							// compiles Sass to CSS, using Node Sass by default
				]
			},
			{
				test: /\.css$/,
				use: [
					"style-loader", 						// creates style nodes from JS strings
					"css-loader"							// translates CSS into CommonJS
				]
			},
			{
				test: /\.(js|jsx|tsx|ts)$/,   // All ts and tsx files will be process by
				loader: 'babel-loader',			// first babel-loader, then ts-loader
				exclude: /node_modules/				// ignore node_modules
			},
			{
				test: /\.(jpe?g|png|gif|svg)$/i,
				loader: 'file-loader',
				options: {
					name: '/public/icons/[name].[ext]'
				}
			}
		]
	},
	devServer: {
		contentBase: "src/",
		historyApiFallback: true,
		port: 8080
	},
	plugins: [
		htmlPlugin,
		new CleanWebpackPlugin({
			verbose: true
    })
	]
};
