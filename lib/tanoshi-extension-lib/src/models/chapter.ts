/**
 * Type represent a chapter of a manga
 * @public
 */
export interface Chapter {
    /**
     * Source Id of the chapter
     */
    sourceId: number;
    /**
     * Title of the chapter
     */
    title: string;
    /**
     * Relative path of the chapter on source
     */
    path: string;
    /**
     * Number of the chapter
     */
    number: number;
    /**
     * Scanlation group who translate the chapter
     */
    scanlator?: string;
    /**
     * Date when the chapters
     */
    uploaded: number;
};