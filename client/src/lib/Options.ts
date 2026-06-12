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
