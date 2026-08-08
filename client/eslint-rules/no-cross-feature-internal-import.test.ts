import { RuleTester } from '@typescript-eslint/rule-tester';
import rule from './no-cross-feature-internal-import';

const ruleTester = new RuleTester();

ruleTester.run('no-cross-feature-internal-import', rule, {
	valid: [
		{
			// same feature with alias: allowed
			code: `import AddressFields from '$ui/checkout/internal/AddressFields.svelte';`,
			filename: '/src/ui/checkout/CheckoutForm.svelte'
		},
		{
			// same feature, relative path to own internal/: allowed
			code: `import AddressFields from './internal/AddressFields.svelte';`,
			filename: '/src/ui/checkout/CheckoutForm.svelte'
		},
		{
			// same feature from nested folder, relative to internal/: allowed
			code: `import AddressFields from '../internal/AddressFields.svelte';`,
			filename: '/src/ui/checkout/nested/Form.svelte'
		},
		{
			// importing from shared $ui/shared: allowed (not internal/)
			code: `import Button from '$ui/shared/Button.svelte';`,
			filename: '/src/ui/checkout/CheckoutForm.svelte'
		},
		{
			// importing from shared via relative path: allowed
			code: `import Button from '../shared/Button.svelte';`,
			filename: '/src/ui/checkout/CheckoutForm.svelte'
		},
		{
			// importing from $lib: allowed
			code: `import utils from '$lib/utils.ts';`,
			filename: '/src/ui/checkout/CheckoutForm.svelte'
		}
	],
	invalid: [
		{
			// cross-feature internal import with alias: disallowed
			code: `import AddressFields from '$ui/checkout/internal/AddressFields.svelte';`,
			filename: '/src/ui/billing/BillingForm.svelte',
			errors: [{ messageId: 'crossFeatureInternal' }]
		},
		{
			// cross-feature internal import with relative path: disallowed
			code: `import AddressFields from '../checkout/internal/AddressFields.svelte';`,
			filename: '/src/ui/billing/BillingForm.svelte',
			errors: [{ messageId: 'crossFeatureInternal' }]
		},
		{
			// cross-feature internal from nested folder: disallowed
			code: `import Component from '../../checkout/internal/Component.svelte';`,
			filename: '/src/ui/billing/nested/Form.svelte',
			errors: [{ messageId: 'crossFeatureInternal' }]
		},
		{
			// dynamic internal import, cross-feature with alias
			code: `const c = await import('$ui/checkout/internal/PaymentSummary.svelte');`,
			filename: '/src/ui/billing/BillingForm.svelte',
			errors: [{ messageId: 'crossFeatureInternal' }]
		},
		{
			// dynamic import with relative path, cross-feature
			code: `const c = await import('../checkout/internal/PaymentSummary.svelte');`,
			filename: '/src/ui/billing/BillingForm.svelte',
			errors: [{ messageId: 'crossFeatureInternal' }]
		},
		{
			// importing from outside ui/ folder: disallowed
			code: `import Component from '../ui/checkout/internal/Component.svelte';`,
			filename: '/src/lib/utils.ts',
			errors: [{ messageId: 'crossFeatureInternalFromShared' }]
		}
	]
});
