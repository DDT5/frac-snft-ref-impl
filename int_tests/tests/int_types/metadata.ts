export default interface Metadata {
    token_uri?: string,
    extension?: Extension,
}

interface Extension {
    image?: string,
    image_data?: string,
    external_url?: string,
    description?: string,
    name?: string,
    attributes?: Trait[],
    background_color?: string,
    animation_url?: string,
    youtube_url?: string,
    media?: MediaFile[],
    protected_attributes?: string[],
}

interface Trait {
    display_type?: string,
    trait_type?: string,
    value: string,
    max_value?: string,
}

interface MediaFile {
    file_type?: string,
    extension?: string,
    authentication?: Authentication,
    url: string,
}

interface Authentication {
    key?: string,
    user?: string,
}
