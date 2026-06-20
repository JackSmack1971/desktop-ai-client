import js from '@eslint/js';
import globals from 'globals';
import tsPlugin from '@typescript-eslint/eslint-plugin';
import tsParser from '@typescript-eslint/parser';
import svelte from 'eslint-plugin-svelte';
import svelteConfig from './svelte.config.js';

const tsRecommendedRules = tsPlugin.configs.recommended.rules;

export default [
	{
		ignores: [
			'.claude/',
			'.planning/',
			'build/',
			'.svelte-kit/',
			'dist/',
			'release-evidence/',
			'scripts/',
			'src-tauri/',
			'target/',
			'node_modules/',
		],
	},
	js.configs.recommended,
	...svelte.configs['flat/recommended'],
	{
		languageOptions: {
			globals: {
				$bindable: 'readonly',
				$derived: 'readonly',
				$effect: 'readonly',
				$host: 'readonly',
				$props: 'readonly',
				$state: 'readonly',
				...globals.browser,
				...globals.node,
			},
		},
	},
	{
		files: ['**/*.{js,ts}'],
		languageOptions: {
			parser: tsParser,
			parserOptions: {
				ecmaVersion: 'latest',
				sourceType: 'module',
			},
		},
		plugins: {
			'@typescript-eslint': tsPlugin,
		},
		rules: {
			'no-unused-vars': 'off',
			...tsRecommendedRules,
			'@typescript-eslint/no-unused-vars': [
				'error',
				{
					argsIgnorePattern: '^_',
					caughtErrorsIgnorePattern: '^_',
					varsIgnorePattern: '^_',
				},
			],
		},
	},
	{
		files: ['**/*.svelte'],
		languageOptions: {
			parserOptions: {
				extraFileExtensions: ['.svelte'],
				parser: tsParser,
				svelteConfig,
			},
		},
		plugins: {
			'@typescript-eslint': tsPlugin,
		},
		rules: {
			'no-unused-vars': 'off',
			'@typescript-eslint/no-unused-vars': [
				'error',
				{
					argsIgnorePattern: '^_',
					caughtErrorsIgnorePattern: '^_',
					varsIgnorePattern: '^_',
				},
			],
		},
	},
];
