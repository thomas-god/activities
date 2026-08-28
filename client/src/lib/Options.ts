export type Some<T> = {
	_kind: 'Some';
	value: T;
};

export type None = {
	_kind: 'None';
};

export type Option<T> = None | Some<T>;

export function isSome<T>(value: Option<T>): value is Some<T> {
	return value._kind === 'Some';
}

export function isSomeAnd<T>(value: Option<T>, predicate: (other: T) => boolean): value is Some<T> {
	if (value._kind === 'None') {
		return false;
	}
	return predicate(value.value);
}

export function isNone<T>(value: Option<T>): value is None {
	return value._kind === 'None';
}

export function some<T>(value: T): Option<T> {
	return {
		_kind: 'Some',
		value
	};
}

export function none<T>(): Option<T> {
	return {
		_kind: 'None'
	};
}

/**
 * Sets the inner value of an option inplace, making it a Some<T>.
 * Useful to avoid breaking Svelte's state reference link when working with contexts,
 * see https://svelte.dev/docs/svelte/context#Using-context-with-state.
 */
export function setInnerValue<T>(value: Option<T>, inner: T): value is Some<T> {
	value._kind = 'Some';
	(value as Some<T>).value = inner;
	return true;
}

export function map<T, U>(value: Option<T>, closure: (v: T) => U): Option<U> {
	if (value._kind === 'None') {
		return none();
	}
	return some(closure(value.value));
}

export function unwrapOr<T>(value: Option<T>, defaultValue: T): T {
	return isSome(value) ? value.value : defaultValue;
}

export function unwrap<T>(value: Option<T>): T {
	if (isNone(value)) {
		throw 'Unwrap a none value';
	}
	return value.value;
}

export function asOption<T>(value: T | null): Option<T> {
	return value === null ? none() : some(value);
}
