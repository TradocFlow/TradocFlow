#[cfg(test)]
mod tests {
    use crate::{i18n, Language};

    #[test]
    fn test_i18n_system() {
        // Initialize the i18n system
        i18n::init();
        
        // Test English (default)
        i18n::set_language(Language::English);
        assert_eq!(i18n::t("app.name"), "Publication System");
        assert_eq!(i18n::t("nav.documents"), "Documents");
        assert_eq!(i18n::t("ui.save"), "Save");
        
        // Test German
        i18n::set_language(Language::German);
        assert_eq!(i18n::t("app.name"), "Publikationssystem");
        assert_eq!(i18n::t("nav.documents"), "Dokumente");
        assert_eq!(i18n::t("ui.save"), "Speichern");
        
        // Test French
        i18n::set_language(Language::French);
        assert_eq!(i18n::t("app.name"), "Système de Publication");
        assert_eq!(i18n::t("nav.documents"), "Documents");
        assert_eq!(i18n::t("ui.save"), "Enregistrer");
        
        // Test Spanish
        i18n::set_language(Language::Spanish);
        assert_eq!(i18n::t("app.name"), "Sistema de Publicación");
        assert_eq!(i18n::t("nav.documents"), "Documentos");
        assert_eq!(i18n::t("ui.save"), "Guardar");
        
        // Test Italian
        i18n::set_language(Language::Italian);
        assert_eq!(i18n::t("app.name"), "Sistema di Pubblicazione");
        assert_eq!(i18n::t("nav.documents"), "Documenti");
        assert_eq!(i18n::t("ui.save"), "Salva");
        
        // Test Dutch
        i18n::set_language(Language::Dutch);
        assert_eq!(i18n::t("app.name"), "Publicatiesysteem");
        assert_eq!(i18n::t("nav.documents"), "Documenten");
        assert_eq!(i18n::t("ui.save"), "Opslaan");
    }
    
    #[test]
    fn test_fallback_translation() {
        i18n::init();
        
        // Test fallback for non-existent key
        i18n::set_language(Language::English);
        let result = i18n::t("non.existent.key");
        // rust-i18n should return the key itself as fallback
        assert_eq!(result, "non.existent.key");
    }
}