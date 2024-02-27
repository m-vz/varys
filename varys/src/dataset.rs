use clap::ValueEnum;

use varys_database::database::interaction::Interaction;

#[derive(ValueEnum, Default, Clone, Debug)]
pub enum DatasetSize {
    /// The full, unchanged dataset.
    #[default]
    Full,
    /// A small dataset with 13 queries, each from a different category.
    Small,
    /// A binary dataset with the two queries *"Call John Doe"* and *"Call Mary Poppins"*.
    Binary,
}

impl DatasetSize {
    /// Filter out all interactions that should not be used for this dataset size.
    ///
    /// # Arguments
    ///
    /// * `interactions`: The interactions to filter.
    pub fn filter(&self, interactions: Vec<Interaction>) -> Vec<Interaction> {
        if let DatasetSize::Full = self {
            return interactions;
        }

        interactions
            .into_iter()
            .filter(|interaction| self.queries().contains(&interaction.query.as_str()))
            .collect()
    }

    /// All queries that are used for this dataset size.
    ///
    /// This returns an empty vector if the dataset size is `DatasetSize::Full`.
    pub fn queries(&self) -> Vec<&str> {
        match self {
            DatasetSize::Full => vec![],
            DatasetSize::Small => vec![
                "Hey Siri. What is the factorial of 6?", // mathematics
                "Hey Siri. What day was 90 days ago?",   // time
                "Hey Siri. What is the temperature in living room?", // home
                "Hey Siri. Any missed calls?",           // calls
                "Hey Siri. Read Calendar",               // calendar
                "Hey Siri. Remind me to wash the car",   // reminders
                "Hey Siri. How far is New York from Boston", // maps
                "Hey Siri. How old is Ian McKellen?",    // trivia
                "Hey Siri. What’s the temperature outside?", // weather
                "Hey Siri. Translate car from English to Spanish", // translation
                "Hey Siri. Roll a die",                  // randomness
                "Hey Siri. Is there a God?",             // banter
                "Hey Siri. What’s 2330 dollars in euros?", // conversion
            ],
            DatasetSize::Binary => vec!["Hey Siri. Call John Doe", "Hey Siri. Call Mary Poppins"],
        }
    }
}
