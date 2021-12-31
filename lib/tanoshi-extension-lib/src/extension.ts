import { Source } from './models/source';
import { Manga } from './models/manga'
import { Chapter } from './models/chapter';
import { Input } from './models/input';
import { HeaderMap } from './models/headers';

/**
 * Extension interface to build extension
 */
export abstract class Extension implements Source {
    readonly abstract id: number;
    readonly abstract name: string;
    readonly abstract url: string;
    readonly abstract version: string;
    readonly abstract icon: string;
    readonly abstract languages: string;
    readonly abstract nsfw: boolean;

    _preferences: Input[] = [];

    public get preferences(): Input[] {
        return this._preferences;
    }

    public set preferences(inputs: Input[]) {
        let saved = new Map<String, Input>();
        for (var pref of inputs) {
            saved.set(`${pref.type}(${pref.name})`, pref);
        }
        this._preferences = this.preferences.map((field) => {
            let f = saved.get(`${field.type}(${field.name})`)
            if (f) {
                field = f;
            }
            return field;
        });
    }

    public headers(): HeaderMap {
        return <HeaderMap>{};
    }

    public filterList(): Input[] {
        return []
    }

    constructor() { }

    /**
     * 
     * @param page 
     * @returns manga list or undefined
     */
    abstract getPopularManga(page: number): Promise<Manga[]>;

    /**
     * 
     * @param page 
     * @returns manga list or undefined
     */
    abstract getLatestManga(page: number): Promise<Manga[]>;

    /**
     * 
     * @param page 
     * @param query 
     * @param filter 
     * @returns manga list or undefined
     */
    abstract searchManga(page: number, query?: string, filter?: Input[]): Promise<Manga[]>;

    /**
     * 
     * @param path to manga 
     * @returns manga or undefined
     */
    abstract getMangaDetail(path: string): Promise<Manga>;

    /**
     * 
     * @param path to chapters 
     * @returns chapter list or undefined
     */
    abstract getChapters(path: string): Promise<Chapter[]>;

    /**
     * 
     * @param path to chapter 
     * @returns url list of images or undefined
     */
    abstract getPages(path: string): Promise<string[]>;
}