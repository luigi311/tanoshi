
/**
 * Type represents a Manga/Series
 * @public
 */
export interface Manga {
    sourceId: number;
    /**
     * Title of the manga
     */
    title: string;
    /**
     * Authors of the manga
     */
    author: string[];
    /**
     * Genres/Tags of the manga
     */
    genre: string[];
    /**
     * Status of the manga
     */
    status?: string;
    /**
     * Description/Synopsis of the manga
     */
    description?: string;
    /**
     * Relative path of the manga on source
     */
    path: string;
    /**
     * Absolute URL for the cover image of the manga
     */
    coverUrl: string;
};