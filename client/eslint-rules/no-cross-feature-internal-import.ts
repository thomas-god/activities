import { ESLintUtils, TSESTree } from '@typescript-eslint/utils';
import * as path from 'path';

const FEATURE_INTERNAL_RE = /ui\/([^/]+)\/internal\//;
const FEATURE_RE = /ui\/([^/]+)\//;

type MessageIds = 'crossFeatureInternal' | 'crossFeatureInternalFromShared';

const createRule = ESLintUtils.RuleCreator(
	(name) => `https://internal-docs.example.com/eslint-rules/${name}`
);

const rule = createRule<[], MessageIds>({
	name: 'no-cross-feature-internal-import',
	meta: {
		type: 'problem',
		docs: {
			description: 'Prevent importing feature-internal components from outside their own feature.'
		},
		schema: [],
		messages: {
			crossFeatureInternal:
				"'{{imported}}' is internal to feature '{{targetFeature}}' and cannot be imported from feature '{{sourceFeature}}'. Duplicate the component locally, or promote it to the shared components/ folder if reuse is genuine.",
			crossFeatureInternalFromShared:
				"'{{imported}}' is internal to feature '{{targetFeature}}' and cannot be imported from outside ui/. Duplicate it locally, or promote it to the shared ui/shared folder."
		}
	},
	defaultOptions: [],
	create(context) {
		function normalize(importPath: string, sourceFile: string): string {
			// Normalize path aliases (e.g. SvelteKit's `$ui/...`) so they
			// match the `ui/<name>/internal/` pattern used below.
			if (importPath.startsWith('$ui/')) {
				return importPath.replace(/^\$ui\//, 'ui/');
			}

			// Handle relative paths: resolve them relative to the source file
			if (importPath.startsWith('./') || importPath.startsWith('../')) {
				const sourceDir = path.dirname(sourceFile);
				const resolvedPath = path.join(sourceDir, importPath);
				// Normalize to forward slashes and extract the relevant part
				return resolvedPath.replace(/\\/g, '/');
			}

			// Return as-is for other imports ($lib, node_modules, etc.)
			return importPath;
		}

		function checkImportPath(node: TSESTree.Node, importPath: string): void {
			const sourceFile = context.filename;
			const resolved = normalize(importPath, sourceFile);
			const targetMatch = resolved.match(FEATURE_INTERNAL_RE);
			if (!targetMatch) return; // not importing from an internal/ folder

			const targetFeature = targetMatch[1];
			const sourceMatch = sourceFile.match(FEATURE_RE);
			const sourceFeature = sourceMatch ? sourceMatch[1] : null;

			if (sourceFeature === targetFeature) return; // same feature, allowed

			context.report({
				node,
				messageId: sourceFeature ? 'crossFeatureInternal' : 'crossFeatureInternalFromShared',
				data: {
					imported: importPath,
					targetFeature,
					sourceFeature: sourceFeature ?? '(shared)'
				}
			});
		}

		return {
			ImportDeclaration(node: TSESTree.ImportDeclaration) {
				checkImportPath(node, node.source.value);
			},
			ImportExpression(node: TSESTree.ImportExpression) {
				// dynamic import('...')
				if (node.source.type === 'Literal' && typeof node.source.value === 'string') {
					checkImportPath(node, node.source.value);
				}
			},
			CallExpression(node: TSESTree.CallExpression) {
				// require('...')
				if (
					node.callee.type === 'Identifier' &&
					node.callee.name === 'require' &&
					node.arguments[0]?.type === 'Literal' &&
					typeof node.arguments[0].value === 'string'
				) {
					checkImportPath(node, node.arguments[0].value);
				}
			}
		};
	}
});

export default rule;
