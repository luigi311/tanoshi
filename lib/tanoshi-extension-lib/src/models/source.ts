/**
 * Type representing a Source
 * @public
 */
export interface Source {
    id: number;
    /**
     * Unique name of the source
     */
    name: string;
    /**
     * Base URL of the source
     */
    url: string;
    /**
     * Version of the source
     */
    version: string;
    /**
     * Absolute URL of the source icon
     */
    icon: string;
    /**
     * Languages supported by the source
     */
    languages: 'all' | string | string[];
    /**
     * Indicate nsfw content provided by the source
     */
    nsfw: boolean;
};