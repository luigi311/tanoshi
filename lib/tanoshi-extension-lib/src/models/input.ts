type Type = 'Text' | 'Checkbox' | 'Select' | 'Group' | 'Sort';

export abstract class Input {
    abstract type: Type;
    abstract name: string;

    public equals(others: Input): boolean {
        return (this.type === others.type && this.name === others.name);
    }
}

export class Text extends Input {
    type: Type = 'Text';
    constructor(public name: string, public state?: string) {
        super();
    }
}

export class Checkbox extends Input {
    type: Type = 'Checkbox';
    constructor(public name: string, public state?: boolean) { super(); }
}

export class Select<T> extends Input {
    type: Type = 'Select';
    constructor(public name: string, public values: T[], public state?: number) { super(); }
}

export class Group<T> extends Input {
    type: Type = 'Group';
    constructor(public name: string, public state?: T[]) { super(); }
}

export class Sort<T> extends Input {
    type: Type = 'Sort';
    constructor(public name: string, public values: T[], public state?: { index: number, ascending: boolean }) { super(); }
}