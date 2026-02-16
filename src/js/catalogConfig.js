const DEVELOPER_CATALOG_URL = 'REPLACE_WITH_CATALOG_URL';

export function getCatalogUrl(savedCatalogUrl = '') {
    if (DEVELOPER_CATALOG_URL !== 'REPLACE_WITH_CATALOG_URL' && DEVELOPER_CATALOG_URL.trim()) {
        return DEVELOPER_CATALOG_URL.trim();
    }
    return (savedCatalogUrl || '').trim();
}
