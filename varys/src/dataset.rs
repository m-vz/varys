use clap::ValueEnum;
use std::fmt::{Display, Formatter};
use log::info;

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
        // Log the initial number of interactions
        info!("Starting filter process. Total interactions: {}", interactions.len());
    
        // Define valid greetings
        let valid_greetings = vec!["Hey Siri. ", "Alexa. "];
    
        // Merge each query with each greeting
        let valid_queries: Vec<String> = self.queries().iter()
            .flat_map(|query| {
                valid_greetings.iter().map(move |greeting| format!("{}{}", greeting, query))
            })
            .collect();
    
        // Filter the interactions and log the ones that are kept
        let filtered_interactions: Vec<Interaction> = interactions
            .into_iter()
            .filter(|interaction| {
                valid_queries.iter().any(|valid_query| interaction.query == *valid_query)
            })
            .collect();
    
        // Log the number of filtered interactions
        info!("Filtering complete. Number of interactions kept: {}", filtered_interactions.len());
    
        filtered_interactions
    }

    /// All queries that are used for this dataset size.
    ///
    /// This returns an empty vector if the dataset size is `DatasetSize::Full`.
    pub fn queries(&self) -> Vec<&str> {
        match self {
            DatasetSize::Full => vec![
                // the 13 queries from the small dataset are excluded since their higher number of
                // samples may skew the training results:
                // "What’s 2330 dollars in euros?",
                // "What is the factorial of 6?",
                // "What day was 90 days ago?",
                // "What is the temperature in living room?",
                // "Any missed calls?",
                // "Read Calendar",
                // "Remind me to wash the car",
                // "How far is New York from Boston",
                // "How old is Ian McKellen?",
                // "What’s the temperature outside?",
                // "Translate car from English to Spanish",
                // "Roll a die",
                // "Is there a God?",
                "What are 130 miles in yards?",
                "What's 200 pounds in kilograms?",
                "What's 45 miles per hour in meters per second?",
                "What are 3 gigabytes in megabytes?",
                "Convert 4.2 acres to square meters.",
                "Convert 250 milliliters to cups.",
                "Convert 180 degrees Celsius to Fahrenheit.",
                "Convert 3000 calories to kilojoules.",
                "Convert 75 miles per gallon to kilometers per liter.",
                "What’s 9 plus 53?",
                "What is 2 to the power of 17?",
                "What is the result of 25 to the power of 4?",
                "What is 244 plus 5%?",
                "What is $200 minus 21%?",
                "What is 9 percent of 63?",
                "What is the area of a circle with a radius of 2 meters?",
                "What is the remainder when 27 is divided by 5?",
                "Calculate the hypotenuse of a right triangle with legs 3 and 4.",
                "Find the greatest common divisor of 48 and 36.",
                "What date is 90 days before December 17?",
                "What year is 39 years after 1994?",
                "How many years until 2049?",
                "How many days until Easter?",
                "How many days until Christmas?",
                "What are two hours five minutes and 39 seconds in seconds?",
                "What is the time zone in London?",
                "What time is it in London?",
                "Current time?",
                "Turn the lights blue",
                "Turn off the radio",
                "I’m home",
                "Set the brightness of the downstairs lights to 50%",
                "Lock the front door",
                "Open the garage",
                "John is my brother",
                "That’s not how you say John Doe",
                "Show John Doe",
                "When is John’s birthday?",
                "How old is my brother?",
                "Whose phone is this?",
                "Learn to pronounce my name",
                "Call John",
                "Call 408 555 1212",
                "Call my brother on speakerphone",
                "Call the nearest restaurant",
                "When did my brother call me?",
                "Play voicemail from John",
                "Get my call history",
                "Redial my last number",
                "Call back my last missed call",
                "Any new voicemail?",
                "Play me my latest voicemail",
                "Show me new messages from John Doe",
                "Show me my messages",
                "Read my messages",
                "Text John Doe I’m in a meeting",
                "Message my brother I’ll be late",
                "Send John see you later",
                "Tell John I’m on the way",
                "Ask my brother Where are you?",
                "Any new email from John Doe?",
                "Show me the email from John Doe yesterday",
                "Send an email to John Doe Protocol",
                "Check email",
                "Read my last email",
                "Post to Facebook I’m eating a sandwich",
                "Post to Twitter Happy New Year!",
                "Tweet with my location very hot here",
                "Show me tweets from Twitter",
                "Show me the latest tweets",
                "Schedule an event Party in New York Wednesday at 10 PM",
                "Schedule a meeting at 1 PM tomorrow for 2 hours",
                "Create a recurring event every Saturday at 2:30 PM called Party",
                "Set up a meeting with John for today at 3 PM",
                "Show me my next appointment",
                "Where is my next meeting?",
                "Show me the appointments for this afternoon",
                "What does my calendar look like on Monday?",
                "When am I meeting with John Doe?",
                "Cancel my Party in New York event from tomorrow",
                "Cancel my event with John Doe",
                "Move my Monday meeting with John to 3 o’clock",
                "Remind me on Friday at 10 PM to wash the car",
                "Add Milk to the Grocery list",
                "Remind me to wash the car when I leave home today",
                "Remind me to buy milk next time I’m here",
                "Remind me to wash the car every second week",
                "Delete the reminder wash the car",
                "Show me my Grocery list",
                "Note 12 Dollars for pizza",
                "Note Interesting Movies",
                "Add 10 Dollars for food to Outcomes note",
                "Add Star Wars to Interesting Movies note",
                "Show me my notes",
                "Show me my note Interesting Movies",
                "Show me my notes from last week",
                "Tell me about the traffic in New York",
                "What are some attractions around here?",
                "Where is Big Ben?",
                "Is the Central Park open now?",
                "Distance between here and New York?",
                "How far away is Boston?",
                "What is the nearest restaurant?",
                "Find a Starbucks",
                "Good Mexican restaurants around here",
                "Table for two in Palo Alto tonight",
                "Make a reservation at a romantic Italian restaurant tonight at 7 PM",
                "Show me the reviews for Alexander’s Steakhouse in Cupertino",
                "Turn off my alarm",
                "Delete all alarms",
                "Turn off my Good Morning alarm",
                "Show me my alarms",
                "Is Ian McKellen still alive?",
                "How tall is Ian McKellen?",
                "Where was Ian McKellen born?",
                "Who is Ian McKellen married to?",
                "Who wrote Harry Potter?",
                "Who invented the iPhone?",
                "How far away is the moon?",
                "How high is Mount Everest?",
                "What is the population of Switzerland?",
                "How many calories in a bagel?",
                "How long do dogs live?",
                "How many teeth does a dog have?",
                "What type of Pokémon is Pikachu?",
                "Spell necessary",
                "What’s the weather like?",
                "Do I need an umbrella for tomorrow?",
                "What’s the weather going to be like in Madrid tomorrow?",
                "Is there is a chance of rain tomorrow?",
                "What’s the perceived temperature outside?",
                "What’s the dew point outside?",
                "Is it windy outside?",
                "What’s the pressure outside?",
                "What’s the visibility outside?",
                "What is the KP Index?",
                "How humid is it outside?",
                "When is the sunrise?",
                "When is the sunset tomorrow?",
                "When is the sunrise on Friday?",
                "When is the sunset in New York?",
                "What’s the Apple stock price?",
                "Compare Apple with Alphabet",
                "Define airplane",
                "What is the definition of airplane?",
                "What does the French word maison mean in English?",
                "Find books by Charles Dickens",
                "Find movies by Christopher Nolan",
                "What is the movie Indiana Jones about?",
                "When was Indiana Jones released?",
                "Runtime of Indiana Jones?",
                "Who acted in Indiana Jones?",
                "Movies with Scarlett Johansson",
                "Best thriller movies?",
                "Which movie won Best Picture in 1966?",
                "What movies are playing this evening?",
                "Buy three tickets to see The Lego Movie tonight in Sacramento",
                "Find some movie theaters near my home",
                "Shuffle my gym playlist",
                "What’s this song?",
                "Who sings this?",
                "I like this song",
                "What is the point spread in the NFL game?",
                "How is Chelsea doing?",
                "Results from Liverpool last game?",
                "Who’s going to win the Vikings game?",
                "When is the next Liverpool game?",
                "What Channel is the Royals game on?",
                "When is the Super Bowl?",
                "Flip a coin",
                "Pick a card",
                "Roll a twenty-sided die",
                "Random number between 30 and 60",
                "See you on the seventh",
                "What is 1 million divided by 0?",
                "What is 0 divided by 0?",
                "What is infinity times infinity?",
                "Rock paper scissors",
                "Sudo make me a sandwich",
                "Tell me a joke",
                "Tell haiku",
                "Tell me a tongue twister",
                "Tell me a story",
                "Tell me a poem",
                "Tell me a secret",
                "Tell me a bedtime story",
                "Sing me a lullaby",
                "Beam me up",
                "Guess what",
                "Who’s on first?",
                "Open the pod bay doors",
                "Sing me a song now",
                "When is your birthday?",
                "What’s your sign?",
                "What’s your favourite animal?",
                "What color is your hair?",
                "How much do you weigh?",
                "Are you smart?",
                "Are you perfect?",
                "Do you think I look fat in this?",
                "Will you marry me?",
                "May the force be with you",
                "Can I call you Jarvis?",
                "When do you sleep?",
                "How is it to be you?",
                "Have you seen Star Wars?",
                "What is your favourite colour?",
                "What are you going to be for Halloween?",
                "Do you know pick up lines?",
                "Mirror mirror on the wall, who’s the fairest of them all?",
                "What does the fox say?",
                "Who let the dogs out?",
                "How much wood could a woodchuck chuck if a woodchuck could chuck wood?",
                "What is the airspeed velocity of an unladen swallow?",
                "Why are fire trucks red?",
                "Why did the chicken cross the road?",
                "What is the meaning of life?",
                "When is the end of the world?",
                "What’s the best phone?",
                "Can I borrow some money?",
                "supercalifragilisticexpialidocious",
                "Rap Beatbox",
                "Can I call you Cortana?",
                "You’re the best",
                "Meow",
                "I’m sleepy",
                "How many languages do you speak?",
            ],
            DatasetSize::Small => vec![
                "What is the factorial of 6?", // mathematics
                "What day was 90 days ago?",   // time
                "What is the temperature in living room?", // home
                "Any missed calls?",           // calls
                "Read Calendar",               // calendar
                "Remind me to wash the car",   // reminders
                "How far is New York from Boston", // maps
                "How old is Ian McKellen?",    // trivia
                "What’s the temperature outside?", // weather
                "Translate car from English to Spanish", // translation
                "Roll a die",                  // randomness
                "Is there a God?",             // banter
                "What’s 2330 dollars in euros?", // conversion
            ],
            DatasetSize::Binary => vec!["Call John Doe", "Call Mary Poppins"],
        }
    }
}

impl Display for DatasetSize {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                DatasetSize::Full => "full",
                DatasetSize::Small => "small",
                DatasetSize::Binary => "binary",
            }
        )
    }
}
